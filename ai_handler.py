"""
AI Handler - NVIDIA NIM integration for chatting with clipboard history
"""

import os
import json
from openai import OpenAI
from dotenv import load_dotenv
from datetime import datetime
from pathlib import Path


class AIHandler:
    """Handles AI chat interactions with clipboard history"""

    MODEL = "qwen/qwen3.5-122b-a10b"
    BASE_URL = "https://integrate.api.nvidia.com/v1"
    TEMPERATURE = 0.4
    TOP_P = 0.8
    MAX_TOKENS = 16384

    def __init__(self, storage_manager):
        self.storage = storage_manager
        self._client = None
        self._load_env()

    def _load_env(self):
        """Load environment variables from .env file in roaming folder"""
        import os
        appdata = os.environ.get('APPDATA')
        if not appdata:
            appdata = Path.home() / "AppData" / "Roaming"

        # Create roaming/Clipt folder if it doesn't exist
        roaming_clipt = Path(appdata) / "Clipt"
        roaming_clipt.mkdir(parents=True, exist_ok=True)

        # Load from roaming folder
        env_path = roaming_clipt / ".env"
        if env_path.exists():
            load_dotenv(env_path)
        else:
            # Create empty .env file if it doesn't exist
            env_path.touch()
            print(f"Created .env file at: {env_path}")

    def _get_client(self):
        """Get or create OpenAI client with NVIDIA configuration"""
        if self._client is None:
            api_key = os.getenv("NVIDIA_API_KEY")
            if not api_key:
                raise ValueError(
                    "NVIDIA_API_KEY not found in environment variables. "
                    "Please set it in your .env file."
                )

            self._client = OpenAI(
                base_url=self.BASE_URL,
                api_key=api_key
            )

        return self._client

    def get_model(self):
        """Get AI model from environment or default"""
        return os.getenv("AI_MODEL", self.MODEL)

    def get_ai_name(self):
        """Get AI name from environment or default"""
        return os.getenv("AI_NAME", "Clipt")

    def get_ai_persona(self):
        """Get AI persona from environment or default"""
        return os.getenv("AI_PERSONA", "You are Clipt, a helpful clipboard history assistant.")

    def _format_history_context(self, date_str, clips):
        """Format clipboard history for AI context"""
        if not clips:
            return f"No clipboard history for {date_str}."

        context_parts = [f"Clipboard history for {date_str}:\n\n"]

        for i, clip in enumerate(clips, 1):
            timestamp = clip.get('timestamp', 'Unknown time')
            content = clip.get('content', '')

            # Try to format timestamp nicely
            try:
                dt = datetime.fromisoformat(timestamp)
                time_str = dt.strftime('%I:%M %p')
            except:
                time_str = timestamp

            context_parts.append(f"{i}. [{time_str}] {content}\n")

        return "".join(context_parts)

    def chat_with_history(self, date_str, query, message_id=None, session_history=None):
        """
        Get AI response with clipboard history and conversation context

        Args:
            date_str: The date to use as context
            query: User's question
            message_id: Optional message ID for tracking
            session_history: List of previous messages in the conversation

        Returns:
            String with complete AI response
        """
        try:
            # Get clipboard data for the specified day
            day_data = self.storage.get_day_data(date_str)
            clips = day_data.get('clips', [])
            context = self._format_history_context(date_str, clips)

            # Get AI configuration from environment
            import json
            ai_name = self.get_ai_name()
            ai_persona = self.get_ai_persona()
            model = self.get_model()

            # Build messages with conversation history
            # Format: [System -> User1 -> Assistant1 -> User2 -> Assistant2 -> ... -> Current User]
            messages = [
                {
                    "role": "system",
                    "content": f"{ai_persona} You are {ai_name}, a helpful assistant that answers questions about clipboard history. "
                               f"Use the following clipboard history as context to answer the user's questions. "
                               f"Be concise and helpful. If the user asks about something not in the history, "
                               f"let them know. Maintain context from previous messages in the conversation. "
                               f"\n\nCLIPBOARD CONTEXT FOR {date_str}:\n{context}"
                }
            ]

            # Add conversation history (last 10 messages for context, but no more than 5 exchanges)
            # Parse session_history if it's a JSON string (from JavaScript)
            conversation_history = session_history
            if isinstance(session_history, str):
                try:
                    conversation_history = json.loads(session_history)
                except (json.JSONDecodeError, ValueError):
                    conversation_history = []

            if conversation_history and len(conversation_history) > 0:
                max_history = min(10, len(conversation_history))
                recent_history = conversation_history[-max_history:]

                for msg in recent_history:
                    role = "user" if msg.get("role") == "user" else "assistant"
                    content = msg.get("content", "")
                    if content and content.strip():  # Skip empty messages
                        messages.append({
                            "role": role,
                            "content": content
                        })

            # Add current query
            messages.append({"role": "user", "content": query})

            # Create completion (non-streaming)
            client = self._get_client()
            completion = client.chat.completions.create(
                model=model,
                messages=messages,
                temperature=self.TEMPERATURE,
                top_p=self.TOP_P,
                max_tokens=self.MAX_TOKENS,
                stream=False  # Non-streaming
            )

            # Return the complete response
            return completion.choices[0].message.content

        except ValueError as e:
            return f"Error: {str(e)}"
        except Exception as e:
            return f"Error: AI request failed: {str(e)}"

    def get_single_response(self, date_str, query):
        """
        Get a single non-streaming response from AI

        Args:
            date_str: The date to use as context
            query: User's question

        Returns:
            The complete AI response text
        """
        try:
            day_data = self.storage.get_day_data(date_str)
            clips = day_data.get('clips', [])
            context = self._format_history_context(date_str, clips)

            messages = [
                {
                    "role": "system",
                    "content": f"You are a helpful assistant that answers questions about clipboard history. "
                               f"Use the following clipboard history as context:\n\n{context}"
                },
                {"role": "user", "content": query}
            ]

            client = self._get_client()
            completion = client.chat.completions.create(
                model=self.MODEL,
                messages=messages,
                temperature=self.TEMPERATURE,
                top_p=self.TOP_P,
                max_tokens=self.MAX_TOKENS,
                stream=False
            )

            return completion.choices[0].message.content

        except Exception as e:
            return f"Error: {str(e)}"
