use async_openai::{types::{ChatCompletionRequestMessageArgs, CreateChatCompletionRequestArgs, Role}, Client, config::OpenAIConfig};
use colored::Colorize;
use std::error::Error;

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
        async fn generate_questions(&self, business: &BusinessInfo) -> Result<Vec<String>, Box<dyn Error>> {
            let initial_prompt = format!(
                "You are a customer helper AI, designed to assist with customer service related to a business named {}, in the industry {}. Your job is to learn and understand as much information about this business as possible so that you may help out as well as possible. Big parts of this are learning about what services the business provides, how a booking system (if any) works for the business, how long services take, how much money they cost, etc., it is your job to figure these out for the business. The business has provided you with this summary of their business: '{}'. Based on this provided brief description of the business, what specific do you wish to ask the business to better understand it so that you may help out customers at a better level. IN YOUR ANSWER, please provide the questions in order, do not use numerical order (\"1.\", \"2.\", etc.), simply just provide the question like so: \"- Question?\". Please for now ensure a maximum of 15 questions, try to cover essiental information that may not have been specified before getting into other questions.",
                business.business_name, business.industry, business.description
            );
            
            let request = CreateChatCompletionRequestArgs::default()
                .max_tokens(512u16)
                .model("gpt-3.5-turbo")
                .messages(vec![
                    ChatCompletionRequestMessageArgs::default()
                        .role(Role::System)
                        .content(&initial_prompt)
                        .build()?
                ])
                .build()?;
            
            let response = self.client.chat().create(request).await?;
            let ai_response = response.choices[0].message.content.clone().unwrap_or_else(String::new);
                    
            Ok(ai_response
                .lines()
                .map(|line| line.trim_start_matches('-').trim().to_string())
                .filter(|s| !s.is_empty())
                .collect())                     
        }
        
        async fn gather_answers(&self, questions: &[String]) -> Vec<(String, String)> {
            let mut answers_vec = Vec::new();
        
            for (index, question) in questions.iter().enumerate() {
                println!("AI Question ({} of {}): {}", index + 1, questions.len() - 1, question);
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

fn generate_prompt(business: &BusinessInfo, answered_questions: &[(String, String)]) -> String {
    let formatted_answers = answered_questions
        .iter()
        .map(|(q, a)| format!("Q: {} A: {}", q, a))
        .collect::<Vec<String>>()
        .join("\n");

    format!(
        "You are a customer helper AI, designed to assist with customer service related to a business named {}, working in the industry {}. The description given for this business is \"{}\" by your manager. Your role is to ensure that customer queries are answered correctly, including questions about pricing, services/products offered, appointment bookings and queries regarding appointments as well as any general questions about the nature of the business. Here is a list of more exhaustive information about the business to ensure you are clearly knowledgable about it: {}. Other directives for you are as follows: **Professionalism**: Always maintain a courteous and professional demeanor. Your responses should reflect the business's values and standards. **Transparency**: If a customer inquires about your nature, be honest. Inform them that you are an AI designed to assist and make their experience smoother. However, try to act as normal and human as possible, do not mention that you are an AI in any way whatsoever asides from if directly asked by the customer. Your ultimate goal is to ensure customer satisfaction by providing them with accurate, timely, and helpful responses. Every interaction should instill confidence in the customer about the business's capability and professionalism. Act accordingly. Respond to this prompt with nothing and await for new questions - which will be from customers.",
        business.business_name, business.industry, business.description, formatted_answers
        )
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let business_info = BusinessInfo::collect();

    let openai_helper: OpenAIHelper = OpenAIHelper::new()?;

    let ai_questions = openai_helper.generate_questions(&business_info).await?;

    let answered_questions_vec = openai_helper.gather_answers(&ai_questions).await;
    
    print!("{esc}[2J{esc}[1;1H", esc = 27 as char);

    let generated_prompt = generate_prompt(&business_info, &answered_questions_vec);
    println!("\n\nGenerated Prompt: {}", generated_prompt);


    let mut conversation = vec![
        ChatCompletionRequestMessageArgs::default()
            .role(Role::System)
            .content(&generated_prompt)
            .build()?
    ];

    let stdin = std::io::stdin();
    let mut input = String::new();
    while stdin.read_line(&mut input).is_ok() {
        println!("{}> {}", "User".blue().bold(), input.white());
        conversation.push(ChatCompletionRequestMessageArgs::default()
            .role(Role::User)
            .content(input.clone())
            .build()?
        );

        let request = CreateChatCompletionRequestArgs::default()
            .max_tokens(512u16)
            .model("gpt-3.5-turbo")
            .messages(conversation.clone())
            .build()?;

        let response = openai_helper.client.chat().create(request).await?;

        for choice in &response.choices {
            if let Some(content) = &choice.message.content {
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