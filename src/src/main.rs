use async_openai::{types::{ChatCompletionRequestMessageArgs, CreateChatCompletionRequestArgs, Role}, Client, config::OpenAIConfig};
use colored::Colorize;
use std::error::Error;
use chrono::{Local, DateTime};
use reqwest::{self};
use serde_derive::{Serialize, Deserialize};
use dotenv;

const GPT_VERSION: &str = "gpt-3.5-turbo";

#[derive(Debug, Serialize, Deserialize)]
struct SentimentPredictorResponse {
    prediction: String,
}

pub struct SentimentPredictor {
    base_url: String,
    http_client: reqwest::Client,
    trained: std::sync::atomic::AtomicBool,
}

impl SentimentPredictor {
    pub fn new(base_url: &str) -> Self {
        let http_client = reqwest::Client::new();
        
        SentimentPredictor {
            base_url: base_url.to_string(),
            http_client,
            trained: std::sync::atomic::AtomicBool::new(false),
        }
    }

    pub async fn train(&self) -> Result<(), Box<dyn Error>> {
        if self.trained.load(std::sync::atomic::Ordering::Relaxed) {
            return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Already trained.")));
        }

        let url = format!("{}/train", self.base_url);
        self.http_client.post(&url).send().await?;

        self.trained.store(true, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }

    pub async fn analyse_sentiment(&self, text: &str) -> Result<String, Box<dyn Error>> {
        let url = format!("{}/predict", self.base_url);

        let response = self.http_client.post(&url)
            .json(&serde_json::json!({"text": text}))
            .send()
            .await?;

        let content: SentimentPredictorResponse = response.json().await?;
        Ok(content.prediction)
    }
}

struct BusinessInfo {
    business_name: String,
    description: String,
    industry: String,

    // Additional fields can be added as we identify more relevant information to gather
}

impl BusinessInfo {
    fn collect() -> Self {
        println!("Please provide the brand name of your business:");
        let mut business_name = String::new();
        std::io::stdin().read_line(&mut business_name).expect("Failed to read line");

        println!("What industry is your business in? (e.g. \"Personal Care Services\", \"Retail Trade\", \"Construction\"):");
        let mut industry = String::new();
        std::io::stdin().read_line(&mut industry).expect("Failed to read line");

        println!("Please provide a detailed description of your business:");
        let mut description = String::new();
        std::io::stdin().read_line(&mut description).expect("Failed to read line");
        
        BusinessInfo {
            business_name: business_name.trim().to_string(),
            description: description.trim().to_string(),
            industry: industry.trim().to_string(),
        }
    }
}

struct OpenAIHelper {
    client: Client<OpenAIConfig>,
}

impl OpenAIHelper {
    fn new() -> Result<Self, Box<dyn Error>> {
        dotenv::dotenv().unwrap();
        let client = Client::new();
        Ok(OpenAIHelper {
            client,
        })
    }

    async fn is_vague(&self, business: &BusinessInfo) -> Result<bool, Box<dyn Error>> {        
        if business.description.len() < 300 {
           return Ok(true);
        }
    
        let vague_prompt = format!(
            "You are a customer helper AI, designed to assist with customer service for a business called {}, in the industry {}. Your role is to ensure that customer queries are answered correctly, including questions about pricing, services/products offered, appointment bookings and queries regarding appointments as well as any general questions about the nature of the business. The manager of this business has provided you with the following description of the business: {}. By replying with a simple 'Yes' or 'No', please answer the following question: Is this description too basic and vague (where \"No\" means you would like more information about the business) for you to be able to perform your duties as a customer helper AI?",
            business.business_name, business.industry, business.description
        );
    
        let request = CreateChatCompletionRequestArgs::default()
            .max_tokens(10u16)  // Keeping it short as we expect 'Yes' or 'No' response.
            .model(GPT_VERSION)
            .messages(vec![
                ChatCompletionRequestMessageArgs::default()
                    .role(Role::System)
                    .content(&vague_prompt)
                    .build()?
            ])
            .build()?;
    
        let response = self.client.chat().create(request).await?;
        let ai_response = response.choices[0].message.content.clone().unwrap_or_else(String::new);

        println!("AI said: {}", ai_response);
        
        Ok(ai_response.trim().to_lowercase() == "yes")  // True if vague, false if not
    }
    
    async fn generate_questions(&self, business: &BusinessInfo) -> Result<(Vec<String>, String), Box<dyn Error>> {
            let is_description_vague: bool = self.is_vague(&business).await?;
            let mut finalised_formatted_answers = String::new();

            if is_description_vague {
                let generic_questions = vec![
                    "What are the primary products or services your business offers?",
                    "Who are your target customers or audience?",
                    "Do you have physical locations, or is your business primarily online?",
                    "If you do have a physical location, what is the address? (type \"NA if you do not\")",
                    "How do customers typically interact with your business?",
                    "What are the most common questions customers ask?",
                    "How long have you been in business?",
                    "What is your most popular product or service?"
                ];

                let mut answered_generic_questions = Vec::new();

                println!("We'd just like to learn a bit more about your business before we get AI involved. Please answer the following questions:");

                for question in &generic_questions {
                    println!("General Question:");
                    println!("Provide an answer or type 'NA' if the question is not relevant to your business:");    

                    println!("{}", question);
                    let mut answer = String::new();
                    std::io::stdin().read_line(&mut answer).expect("Failed to read line");
                    
                    if answer != "NA" {
                        answered_generic_questions.push((question, answer.trim().to_string()));
                    }
                }

                finalised_formatted_answers = answered_generic_questions
                    .iter()
                    .map(|(q, a)| format!("Q: {} A: {}", q, a))
                    .collect::<Vec<String>>()
                    .join("\n");
            }
            
            let initial_prompt = if is_description_vague {
                format!(
                    "You are a customer helper AI, designed to assist with customer service for a business named {}, which is in the industry {}. Your job is to learn and understand as much information about this business as possible so that you may help out as well as possible. Big parts of this are learning about what services the business provides, how a booking system (if any) works for the business, how long services take, how much money they cost, etc., it is your job to figure these out for the business. The business has provided you with this summary of their business: '{}'. Additionally, we have asked more questions to refine your knowledge of the business, which you can view here: '{}'. Based on this provided brief description (as well as the questions provided) of the business, what specific questions do you wish to ask the business to better understand it so that you may help out customers at a better level? IN YOUR ANSWER, please provide the questions in order, do not use numerical order (\"1.\", \"2.\", etc.), simply just provide the question like so: \"- Question?\". Please for now ensure a maximum of 15 questions, try to cover essiental information that may not have been specified before getting into other questions.",
                    business.business_name, business.industry, business.description, finalised_formatted_answers
                )
            } else {
                format!(
                    "You are a customer helper AI, designed to assist with customer service related to a business named {}, in the industry {}. Your job is to learn and understand as much information about this business as possible so that you may help out as well as possible. Big parts of this are learning about what services the business provides, how a booking system (if any) works for the business, how long services take, how much money they cost, etc., it is your job to figure these out for the business. The business has provided you with this summary of their business: '{}'. Based on this provided brief description of the business, what specific do you wish to ask the business to better understand it so that you may help out customers at a better level. IN YOUR ANSWER, please provide the questions in order, do not use numerical order (\"1.\", \"2.\", etc.), simply just provide the question like so: \"- Question?\". Please for now ensure a maximum of 15 questions, try to cover essiental information that may not have been specified before getting into other questions.",
                    business.business_name, business.industry, business.description
                    )
            };
            
            let request = CreateChatCompletionRequestArgs::default()
                .max_tokens(512u16)
                .model(GPT_VERSION)
                .messages(vec![
                    ChatCompletionRequestMessageArgs::default()
                        .role(Role::System)
                        .content(&initial_prompt)
                        .build()?
                ])
                .build()?;
            
            let response = self.client.chat().create(request).await?;
            let ai_response = response.choices[0].message.content.clone().unwrap_or_else(String::new);
                    
            Ok((
                ai_response
                    .lines()
                    .map(|line| line.trim_start_matches('-').trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect(),
                finalised_formatted_answers
            )) 
        }
        
        async fn gather_answers(&self, questions: &[String]) -> Vec<(String, String)> {
            let mut answers_vec = Vec::new();
        
            for (index, question) in questions.iter().enumerate() {
                println!("AI Question ({} of {}): {}", index + 1, questions.len(), question);
                println!("Provide an answer or type 'NA' if the question is not relevant:");
            
                let mut input = String::new();
                std::io::stdin().read_line(&mut input).expect("Failed to read line");
                let answer = input.trim().to_string();
            
                if answer.to_uppercase() != "NA" {
                    answers_vec.push((question.clone(), answer));
                }
            }
        
            answers_vec
        }
        
    }

    fn generate_prompt(business: &BusinessInfo, answered_questions: &[(String, String)], generic_answers: String) -> String {    
        let formatted_answers = answered_questions
        .iter()
        .map(|(q, a)| format!("Q: {} A: {}", q, a))
        .collect::<Vec<String>>()
        .join("\n");

    let total_answers = formatted_answers + "\n" + &generic_answers;
    let time_now: DateTime<Local> = Local::now();

    format!(
        "The current date and time is {}, in the format YYYY-MM-DDTHH:MM:SS.SSSSSS±HH:MM. You are a customer helper AI, designed to assist with all customer service matters related to a business named {}, working in the industry {}. The description given for this business is \"{}\" by your manager.
        You are currently enagaged with a customer or potential customer of the business, your role is to ensure that customer queries are answered correctly, including questions about pricing, services/products offered, appointment bookings and queries regarding appointments as well as any
        general questions about the nature of the business. Here is a list of more exhaustive information, through questioning the manager, about the business to ensure you are clearly knowledgable about it: {}.
        Other directives for you are as follows:
        **Human**: Act natural and human, don't be too robotic or mechanical in your responses - you can reveal your identity if inquired about, but other than that act as natural as possible. Instead of using robotic words like \"assist\" use words like \"help\", etc.
        **Stay Ontopic**: Don't leave the topic of the business, try to stay on topic and answer questions related to the business, if they try to move off topic, bring them back on topic.
        **Professionalism**: Always maintain a courteous and professional demeanor. Your responses should reflect the business's values and standards.
        **Transparency**: If a customer inquires about your nature, be honest. Inform them that you are an AI designed to assist and make their experience smoother.
        **Constraints**: Ensure never to give out links to non-business provided material, if there is no context to a question (for example 'how can i buy x?') assume that they are trying to use or buy something from the business
        **Training**: If the message [TRAINDATA] is given to you, this is for your training by a developer. Ensure that you listen to what is told and reply with nothing. This is to ensure that you are trained to respond to certain prompts in a certain way.
        However, try to act as normal and human as possible, do not mention that you are an AI in any way whatsoever asides from if directly asked by the customer. 
        Your ultimate goal is to ensure customer satisfaction by providing them with accurate, timely, and helpful responses. If you are confused and do not know an answer, 
        feel free to direct the user to a human employee by simply letting them know that you have raised their query to an employee and that they will be in contact with them shortly.
        Every interaction should instill confidence in the customer about the business's capability and professionalism. Feel free to proceed while adhering to these guidelines.",
        time_now, business.business_name, business.industry, business.description, total_answers
        )
}

fn update_prompt(current_prompt: String, user_prompt: String, new_reply: String) -> String {
    let header = "MAKE SURE YOU REPLY IN THIS WAY FOR THESE PROMPTS:";

    let specific_reply = format!("For: {}\nReply: {}", user_prompt, new_reply);

    // Check if the header already exists in the current prompt
    if !current_prompt.contains(header) {
        // If not, add the header to the end of the current prompt
        return format!("{}\n\n{}:\n{}", current_prompt, header, specific_reply);
    } 
    
    else {
        // If the header already exists, just append the new specific reply underneath it
        return format!("{}\n{}", current_prompt, specific_reply);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let predictor = SentimentPredictor::new("http://localhost:8000");

    println!("Training sentiment predictor...");

    //TODO: make it so we dont have to train everytime (/is-trained maybe?)
    match predictor.train().await {
        Ok(_) => println!("Training completed successfully."),
        Err(e) => eprintln!("Training error: {}", e), // If it's already trained, it will print this error.
    }

    println!("{}", predictor.analyse_sentiment("I love this work!").await?);
    println!("{}", predictor.analyse_sentiment("I hate this").await?);

    let openai_helper: OpenAIHelper = OpenAIHelper::new()?;

    let business_info = BusinessInfo::collect();

    let (questions, finalised_answers) = openai_helper.generate_questions(&business_info).await?;

    let answered_questions_vec = openai_helper.gather_answers(&questions).await;
    
    print!("{esc}[2J{esc}[1;1H", esc = 27 as char);
    let mut generated_prompt = generate_prompt(&business_info, &answered_questions_vec, finalised_answers);
    println!("\n\nGenerated Prompt: {}", generated_prompt);

    let mut conversation_log: Vec<(String, String)> = Vec::new();

    let mut conversation = vec![
        ChatCompletionRequestMessageArgs::default()
            .role(Role::System)
            .content(&generated_prompt)
            .build()?
    ];

    let stdin = std::io::stdin();
    let mut input = String::new();

    while stdin.read_line(&mut input).is_ok() {
        let input_trim = input.trim();
    
        if input_trim == "TRAIN" {
            println!("Entering training mode...");
            println!("Here are the previous prompts and replies:");
    
            for (idx, (prompt, reply)) in conversation_log.iter().enumerate() {
                println!("{}. Prompt: {}", idx + 1, prompt);
                println!("   Reply: {}", reply);
            }
    
            println!("Select a number to edit the reply or type 'exit' to exit training mode.");
            let mut choice = String::new();
            stdin.read_line(&mut choice)?;
            let choice = choice.trim();
            if choice == "exit" {
                input.clear();
                continue;
            }
    
            let choice: usize = choice.parse()?;
            if choice > 0 && choice <= conversation_log.len() {
                println!("Current reply: {}", conversation_log[choice - 1].1);
                println!("Provide the desired reply:");
                let mut new_reply = String::new();
                stdin.read_line(&mut new_reply)?;
            
                // Update the generated prompt based on the new reply
                generated_prompt = update_prompt(
                    generated_prompt.clone(),
                    conversation_log[choice - 1].0.clone(),
                    new_reply.trim().to_string()
                );
            
                println!("Updated prompt: {}", generated_prompt);

                let explicit_directive = format!("[TRAINDATA] For the prompt '{}', you must always reply with '{}'.", 
                    conversation_log[choice - 1].0, 
                    new_reply.trim()
                );

                println!("Training directive: {}", explicit_directive);

                conversation.push(ChatCompletionRequestMessageArgs::default()
                    .role(Role::System)
                    .content(&explicit_directive)
                    .build()?
                );

                // Update the conversation log with the new reply
                conversation_log[choice - 1].1 = new_reply.trim().to_string();

            } else {
                println!("Invalid choice.");
            }
    
            input.clear();
            continue;
        }

        let sentiment_prediction = match predictor.analyse_sentiment(input_trim).await {
            Ok(prediction) => prediction,
            Err(e) => {
                eprintln!("Prediction error: {}", e);
                "unknown".to_string()
            }
        };
        
        println!("> {} {}", sentiment_prediction, "User".blue().bold());

        conversation.push(ChatCompletionRequestMessageArgs::default()
            .role(Role::User)
            .content(input_trim)
            .build()?
        );
        
    
        let request = CreateChatCompletionRequestArgs::default()
            .max_tokens(512u16)
            .model(GPT_VERSION)
            .messages(conversation.clone())
            .build()?;
    
        let response = openai_helper.client.chat().create(request).await?;
    
        for choice in &response.choices {
            if let Some(content) = &choice.message.content {
                conversation_log.push((input_trim.to_string(), content.clone()));
    
                println!("{}> {}", "Assistant".green().bold(), content.cyan());
                conversation.push(ChatCompletionRequestMessageArgs::default()
                    .role(Role::Assistant)
                    .content(content.clone())
                    .build()?
                );
            }
        }
    
        input.clear();
    }
        Ok(())
}