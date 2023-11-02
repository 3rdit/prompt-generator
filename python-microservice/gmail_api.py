import json
import os
import pickle
import base64
from google_auth_oauthlib.flow import InstalledAppFlow
from googleapiclient.discovery import build
from googleapiclient.errors import HttpError
from email.mime.text import MIMEText
from google.auth.transport.requests import Request
from google.oauth2.credentials import Credentials

current_dir = os.path.dirname(os.path.abspath(__file__))
credentials_dir = os.path.join(current_dir, "\\bin\\credential")

class GmailAPI:
    CLIENT_ID = '269656723752-o60qt7t5lfsd16ag1ct0qsl9udqm1bcu.apps.googleusercontent.com'
    CLIENT_SECRET = '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'
    REDIRECT_URI = 'http://localhost:8080'
    
    SCOPES = [
        'https://www.googleapis.com/auth/gmail.readonly',
        'https://www.googleapis.com/auth/gmail.send',
        'https://www.googleapis.com/auth/gmail.modify'  # To mark messages as read
    ]
    
    def __init__(self):            
        self.service = None
    
    def _ensure_service(self):
        if self.service is None:
            raise Exception("GmailAPI: 'service' has not been initialised. Please initialise the service before using any methods.")

    def get_service(self, user_id: str | int):
        if user_id is int:
            user_id = str(user_id)
        
        flow = InstalledAppFlow.from_client_config({
            "installed": {
                "client_id": self.CLIENT_ID,
                "client_secret": self.CLIENT_SECRET,
                "redirect_uri": self.REDIRECT_URI,
                "auth_uri": "https://accounts.google.com/o/oauth2/auth",
                "token_uri": "https://oauth2.googleapis.com/token"
            }
        }, self.SCOPES)

        if os.path.exists(f'bin\\token_{user_id}_.pickle'):
            with open(f'bin\\token_{user_id}_.pickle', 'rb') as token:
                creds = pickle.load(token)
        else:
            creds = None

        if not creds or not creds.valid:
            if creds and creds.expired and creds.refresh_token:
                creds.refresh(Request())
            else:
                creds = flow.run_local_server(port=8080, redirect_uri_trailing_slash=False)
                with open(f'bin\\token_{user_id}_.pickle', 'wb') as token:
                    pickle.dump(creds, token)
                    
        self.service = build('gmail', 'v1', credentials=creds)

    def list_emails(self, num_emails=10):
        self._ensure_service()
    
        try:
            results = self.service.users().messages().list(userId='me', maxResults=num_emails).execute()
            return results.get('messages', [])
        except HttpError as error:
            print(f'An error occurred: {error}')
            raise error
        
    def list_unread_emails(self, num_emails=10):
        self._ensure_service()
        try:
            results = self.service.users().messages().list(
                userId='me', 
                maxResults=num_emails, 
                q='is:unread'
            ).execute()
            
            return results.get('messages', [])
        except HttpError as error:
            raise error
        
    def list_emails_with_details(self, message_ids=None):
        """List recent emails with their details."""
        if not message_ids:
            results = self.service.users().messages().list(userId='me', maxResults=10).execute()
            message_ids = [message['id'] for message in results.get('messages', [])]

        emails = []
        for message_id in message_ids:
            email_details = self.get_email_details(message_id)
            if email_details:
                emails.append(email_details)

        return emails

    def send_email(self, to_email, subject, body):
        self._ensure_service()
        
        email = MIMEText(body)
        email['to'] = to_email
        email['subject'] = subject

        encoded_email = base64.urlsafe_b64encode(email.as_bytes()).decode('utf-8')
        try:
            message = self.service.users().messages().send(userId='me', body={'raw': encoded_email}).execute()
            print(f"Email sent to {to_email}. Message Id: {message['id']}")
        except HttpError as error:
            print(f"An error occurred: {error}")

    def create_message(self, sender, to, subject, message_text):
        self._ensure_service()
        """Create a message for an email."""
        message = MIMEText(message_text)
        message['to'] = to
        message['from'] = sender
        message['subject'] = subject
        raw_message = base64.urlsafe_b64encode(message.as_bytes()).decode('utf-8')
        return {'raw': raw_message}

    def send_message(self, service, user_id, message):
        self._ensure_service()

        try:
            message = service.users().messages().send(userId=user_id, body=message).execute()
            print(f"Message Id: {message['id']}")
            return message
        except HttpError as error:
            print(f'An error occurred: {error}')
            return None

    def send_reply(self, service, to_email, original_subject, content):
        self._ensure_service()

        reply_subject = f"RE: {original_subject}"
        message = self.create_message('me', to_email, reply_subject, content)
        self.send_message(service, 'me', message)
        
    def extract_body_from_payload(self, payload):
        if 'parts' in payload:
            for part in payload['parts']:
                part_body = part.get('body', {})
                part_data = part_body.get('data', '')
                part_headers = part.get('headers', [])
                
                is_plain_text = any(header.get('name').lower() == 'content-type' and 
                                    'text/plain' in header.get('value').lower() 
                                    for header in part_headers)
                
                if part_data and is_plain_text:
                    # Decoding the base64-encoded text/plain part
                    return base64.urlsafe_b64decode(part_data.encode('ASCII')).decode('utf-8')
        else:
            # In payload
            body_data = payload.get('body', {}).get('data', '')
            if body_data:
                return base64.urlsafe_b64decode(body_data.encode('ASCII')).decode('utf-8')
        
        return "" # malformed? no body? 
    
    def get_email_details(self, message_id):
        try:
            message = self.service.users().messages().get(userId='me', id=message_id, format='full').execute()
            headers = message['payload']['headers']
            subject = next(header['value'] for header in headers if header['name'].lower() == 'subject')
            from_email = next(header['value'] for header in headers if header['name'].lower() == 'from')
            body = self.extract_body_from_payload(message['payload'])

            return {'subject': subject, 'from': from_email, 'body': body}
        except HttpError as error:
            print(f"An error occurred: {error}")
            return None

    def mark_email_as_read(self, message_id):
        self._ensure_service()
        try:
            # Remove the 'UNREAD' label from the email
            self.service.users().messages().modify(
                userId='me',
                id=message_id,
                body={'removeLabelIds': ['UNREAD']}
            ).execute()
            print(f"Email with id {message_id} marked as read.")
        
        except Exception as e:
            print(f"An error occurred: {e}")
            raise

    def add_label_to_email(self, message_id):
        self._ensure_service()
        label_id = self.create_label_if_not_exists('Astro-Emails')
        
        try:
            # Add the specified label to the email
            self.service.users().messages().modify(
                userId='me',
                id=message_id,
                body={'addLabelIds': [label_id]}
            ).execute()
            print(f"Label with id {label_id} added to email with id {message_id}.")
        except Exception as e:
            print(f"An error occurred: {e}")
            raise
        
    def create_label_if_not_exists(self, label_name):
        self._ensure_service()
        try:
            # Get the list of all labels
            results = self.service.users().labels().list(userId='me').execute()
            labels = results.get('labels', [])
            
            # Check if label already exists
            for label in labels:
                if label['name'].lower() == label_name.lower():
                    print(f"Label '{label_name}' already exists with ID: {label['id']}")
                    return label['id']
            
            # If not found, create it
            label_body = {
                'name': label_name,
                'messageListVisibility': 'show',
                'labelListVisibility': 'labelShow',
            }
            
            created_label = self.service.users().labels().create(userId='me', body=label_body).execute()
            print(f"Label '{label_name}' created with ID: {created_label['id']}")
            return created_label['id']
        
        except Exception as e:
            print(f"An error occurred: {e}")
            raise

if __name__ == '__main__':
    gmail = GmailAPI()
    gmail.get_service(1)
    print(gmail.get_email_details(gmail.list_unread_emails(1)))