use std::error::Error;
use async_openai::{types::{ChatCompletionRequestMessageArgs, CreateChatCompletionRequestArgs, Role}, Client};
use colored::Colorize;
use async_openai::config::OpenAIConfig;

struct BusinessInfo {
    name: String,
    description: String,
    contact_info: ContactInfo,
}

struct ContactInfo {
    email: String,
    phone: Option<String>,
    address: Option<String>,
}

fn collect_business_info() -> BusinessInfo {
    BusinessInfo {
        name: get_input("Enter the business name: "),
        description: get_input("What is your business? Please inform me in as detailed as possible so I can understand!"),
        contact_info: collect_contact_info(),
    }
}

fn collect_contact_info() -> ContactInfo {
    ContactInfo {
        email: get_input("Enter the business contact email: "),
        phone: Some(get_input("Enter the business contact phone (leave blank if none): ")),
        address: Some(get_input("Enter the business address (leave blank if none): ")),
    }
}

fn get_input(prompt: &str) -> String {
    println!("{}", prompt);
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).expect("Failed to read line");
    input.trim().to_string()
}

async fn generate_ai_questions(client: &Client<OpenAIConfig>, business: &BusinessInfo) -> Result<Vec<String>, Box<dyn Error>> {
    let initial_prompt = format!(
        "Based on the provided description of '{}', what questions should we ask to better understand the business and its operations?",
        business.description
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

    let response = client.chat().create(request).await?;
    let ai_response = response.choices[0].message.content.clone().unwrap_or_else(String::new);

    // Splitting the AI response into individual questions
    Ok(ai_response.split('.').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
}

async fn collect_ai_answers(client: &Client<OpenAIConfig>, questions: &[String]) -> Result<Vec<String>, Box<dyn Error>> {
    let mut answers = Vec::new();
    for question in questions {
        let mut conversation = vec![
            ChatCompletionRequestMessageArgs::default()
                .role(Role::System)
                .content(question)
                .build()?
        ];

        println!("{}> {}", "Assistant".green().bold(), question.cyan());

        let mut input = String::new();
        std::io::stdin().read_line(&mut input).expect("Failed to read line");
        println!("{}> {}", "User".blue().bold(), input.white());

        answers.push(input.trim().to_string());
    }

    Ok(answers)
}

fn refine_description(business: &BusinessInfo, questions: &[String], answers: &[String]) -> String {
    format!(
        "{}\n\nAdditional Information:\n{}",
        business.description,
        questions.iter().zip(answers.iter()).map(|(q, a)| format!("{}: {}", q, a)).collect::<Vec<_>>().join("\n")
    )
}

fn generate_prompt(business: &BusinessInfo, ai_questions: &[String]) -> String {
    let mut prompt = format!(
        "Hello! You're now assisting with matters related to {}. {}. \
        As a personal assistant for the business, you have two primary responsibilities: \
        1. **Appointment Bookings**: You can handle appointment bookings for clients. When a client requests an appointment, \
        acknowledge their request, provide them with a confirmation (even if it's a simulated one for now), and ensure they have all the necessary details for their appointment. \
        2. **Answering Questions**: You're equipped with information about the business and its services. Answer any questions clients might have, \
        whether it's about services, pricing, hours of operation, or any other business-related topic. \
        Always be courteous, professional, and helpful. Your goal is to provide clients with a seamless and pleasant experience, \
        making them feel as if they're interacting with a well-informed human assistant. \
        If a client ever asks about your nature, be honest and let them know you're an AI designed to assist them. \
        However, always prioritize their needs and questions. Let's ensure every client interaction is positive and productive!\n\n",
        business.name, business.description
    );

    prompt += "AI Generated Questions:\n";
    for question in ai_questions {
        prompt += &format!("- {}\n", question);
    }

    prompt += "\nContact Information:\n";
    prompt += &format!("- Email: {}\n", business.contact_info.email);
    if let Some(phone) = &business.contact_info.phone {
        prompt += &format!("- Phone: {}\n", phone);
    }
    if let Some(address) = &business.contact_info.address {
        prompt += &format!("- Address: {}\n", address);
    }

    prompt += "\nFeel free to proceed while adhering to these guidelines.\n";

    println!("{}", prompt.bold().green());
    
    prompt
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv::dotenv().unwrap();
    let client: Client<OpenAIConfig> = Client::new();    

    let business_info = collect_business_info();

    // Generate AI questions based on the business description
    let questions = generate_ai_questions(&client, &business_info).await?;

    let mut answers: Vec<String> = Vec::new();

    // Ask the AI generated questions to the user and collect the answers
    for (index, question) in questions.iter().enumerate() {
        // Remove any leading digits and periods from the question
        let cleaned_question = question.trim_start_matches(|c: char| c.is_numeric() || c == '.').trim();
        
        // If the cleaned question is empty, skip to the next question
        if cleaned_question.is_empty() {
            continue;
        }
        
        println!("AI> {}. {}", index + 1, cleaned_question);
        let answer = get_input("Your answer: ");
        answers.push(answer);
    }
    
    let extended_description = format!("{}\n{}", business_info.description, answers.join("\n"));

    let updated_business_info = BusinessInfo {
        description: extended_description,
        ..business_info
    };

    let prompt = generate_prompt(&updated_business_info, &questions);

    let mut conversation = vec![
        ChatCompletionRequestMessageArgs::default()
            .role(Role::System)
            .content(&prompt)
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

        let response = client.chat().create(request).await?;

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

