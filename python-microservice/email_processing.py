import asyncio
from fastapi import BackgroundTasks, HTTPException
from app import app, gmail

async def process_incoming_emails():
    while True:
        try:
            unprocessed_emails = gmail.list_unread_emails()  # Fetch last 10 emails for processing
        except Exception as e:
            raise HTTPException(status_code=500, detail=str(e))

        for email in unprocessed_emails:
            message_id = email['id']
            email_details = gmail.get_email_details(message_id)

            if email_details:
                #TODO: implement AI into processing the request
                response_body = "Thank you for reaching out. We're processing your request."

                try:
                    gmail.send_reply(gmail.service, email_details['from'], email_details['subject'], response_body)
                    gmail.mark_email_as_read(message_id)
                    gmail.add_label_to_email(message_id)
                    
                except Exception as e:
                    print(f"An error occurred when sending a reply: {e}")
                    
        await asyncio.sleep(5)
    
@app.post("/gmail/process_incoming_emails/")
async def start_processing_emails(background_tasks: BackgroundTasks):
    background_tasks.add_task(process_incoming_emails)
    return {"message": "Started processing incoming emails in the background"}