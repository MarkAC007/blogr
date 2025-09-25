//! Newsletter management and configuration

use anyhow::{Context, Result};
use std::env;
use std::path::Path;

use super::composer::{Newsletter, NewsletterComposer};
use super::database::NewsletterDatabase;
use super::fetcher::EmailFetcher;
use super::sender::NewsletterSender;
use crate::config::{Config, ImapConfig, SmtpConfig};
use crate::content::Post;

pub struct NewsletterManager {
    config: Config,
    database: NewsletterDatabase,
}

impl NewsletterManager {
    /// Create a new newsletter manager
    pub fn new(config: Config, project_root: &Path) -> Result<Self> {
        let db_path = project_root.join(".blogr").join("newsletter.db");

        // Ensure .blogr directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).context("Failed to create .blogr directory")?;
        }

        let database =
            NewsletterDatabase::open(&db_path).context("Failed to open newsletter database")?;

        Ok(Self { config, database })
    }

    /// Check if newsletter functionality is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.newsletter.enabled
    }

    /// Get a reference to the database
    pub fn database(&self) -> &NewsletterDatabase {
        &self.database
    }

    /// Take ownership of the database
    pub fn take_database(self) -> NewsletterDatabase {
        self.database
    }

    /// Get IMAP configuration, either from config file or environment
    pub fn get_imap_config(&self) -> Result<Option<ImapConfig>> {
        if let Some(ref imap_config) = self.config.newsletter.imap {
            return Ok(Some(imap_config.clone()));
        }

        // Try to build from environment variables
        let server = env::var("NEWSLETTER_IMAP_SERVER").ok();
        let port = env::var("NEWSLETTER_IMAP_PORT")
            .ok()
            .and_then(|p| p.parse().ok());
        let username = env::var("NEWSLETTER_IMAP_USERNAME").ok();

        if let (Some(server), Some(port), Some(username)) = (server, port, username) {
            Ok(Some(ImapConfig {
                server,
                port,
                username,
                use_tls: Some(true),
            }))
        } else {
            Ok(None)
        }
    }

    /// Get SMTP configuration, either from config file or environment
    pub fn get_smtp_config(&self) -> Result<Option<SmtpConfig>> {
        if let Some(ref smtp_config) = self.config.newsletter.smtp {
            return Ok(Some(smtp_config.clone()));
        }

        // Try to build from environment variables
        let server = env::var("NEWSLETTER_SMTP_SERVER").ok();
        let port = env::var("NEWSLETTER_SMTP_PORT")
            .ok()
            .and_then(|p| p.parse().ok());
        let username = env::var("NEWSLETTER_SMTP_USERNAME").ok();

        if let (Some(server), Some(port), Some(username)) = (server, port, username) {
            Ok(Some(SmtpConfig {
                server,
                port,
                username,
                use_tls: Some(true),
            }))
        } else {
            Ok(None)
        }
    }

    /// Get IMAP password from environment variable
    pub fn get_imap_password(&self) -> Result<String> {
        env::var("NEWSLETTER_IMAP_PASSWORD")
            .context("NEWSLETTER_IMAP_PASSWORD environment variable not set")
    }

    /// Get SMTP password from environment variable
    pub fn get_smtp_password(&self) -> Result<String> {
        env::var("NEWSLETTER_SMTP_PASSWORD")
            .context("NEWSLETTER_SMTP_PASSWORD environment variable not set")
    }

    /// Fetch subscribers from email inbox
    pub fn fetch_subscribers(&mut self, interactive: bool) -> Result<()> {
        if !self.is_enabled() {
            return Err(anyhow::anyhow!(
                "Newsletter functionality is not enabled. Set newsletter.enabled = true in blogr.toml"
            ));
        }

        let imap_config = self.get_imap_config()?
            .ok_or_else(|| anyhow::anyhow!(
                "IMAP configuration not found. Set up IMAP config in blogr.toml or environment variables"
            ))?;

        let password = if interactive {
            self.prompt_for_password("IMAP")?
        } else {
            self.get_imap_password()?
        };

        println!("Connecting to IMAP server...");
        let mut fetcher = EmailFetcher::new();
        fetcher.connect(&imap_config, &password)?;

        println!("Fetching subscription emails...");
        let emails = fetcher.fetch_subscription_emails()?;

        if emails.is_empty() {
            println!("No new subscription emails found.");
            return Ok(());
        }

        println!("Processing {} emails for subscribers...", emails.len());
        let new_subscribers = fetcher.process_subscribers(&emails, &mut self.database)?;

        if new_subscribers.is_empty() {
            println!("No new subscribers found.");
        } else {
            println!("Found {} new subscribers:", new_subscribers.len());
            for subscriber in &new_subscribers {
                println!("  - {}", subscriber.email);
            }
            println!("\nUse 'blogr newsletter approve' to review and approve these subscribers.");
        }

        // Mark emails as processed
        let email_ids: Vec<u32> = emails.iter().map(|e| e.id).collect();
        fetcher.mark_emails_as_seen(&email_ids)?;

        Ok(())
    }

    /// Prompt user for password securely
    fn prompt_for_password(&self, service: &str) -> Result<String> {
        use std::io::{self, Write};

        print!("Enter {} password: ", service);
        io::stdout().flush()?;

        // In a real implementation, you'd want to use a library like `rpassword`
        // for secure password input. For now, we'll use a simple approach.
        let mut password = String::new();
        io::stdin().read_line(&mut password)?;

        Ok(password.trim().to_string())
    }

    /// Get mutable database reference
    #[allow(dead_code)]
    pub fn database_mut(&mut self) -> &NewsletterDatabase {
        &self.database
    }

    /// Print newsletter configuration status
    pub fn print_status(&self) -> Result<()> {
        println!("Newsletter Configuration Status:");
        println!("================================");
        println!("Enabled: {}", if self.is_enabled() { "✓" } else { "✗" });

        if let Some(ref email) = self.config.newsletter.subscribe_email {
            println!("Subscribe Email: {}", email);
        } else {
            println!("Subscribe Email: Not configured");
        }

        if let Some(ref name) = self.config.newsletter.sender_name {
            println!("Sender Name: {}", name);
        } else {
            println!("Sender Name: Not configured");
        }

        // Check IMAP configuration
        match self.get_imap_config()? {
            Some(ref config) => {
                println!("IMAP Server: {}:{}", config.server, config.port);
                println!("IMAP Username: {}", config.username);
                println!(
                    "IMAP Password: {}",
                    if self.get_imap_password().is_ok() {
                        "✓ Set"
                    } else {
                        "✗ Not set"
                    }
                );
            }
            None => {
                println!("IMAP Configuration: Not configured");
            }
        }

        // Check SMTP configuration
        match self.get_smtp_config()? {
            Some(ref config) => {
                println!("SMTP Server: {}:{}", config.server, config.port);
                println!("SMTP Username: {}", config.username);
                println!(
                    "SMTP Password: {}",
                    if self.get_smtp_password().is_ok() {
                        "✓ Set"
                    } else {
                        "✗ Not set"
                    }
                );
            }
            None => {
                println!("SMTP Configuration: Not configured");
            }
        }

        // Database statistics
        let total_subscribers = self.database.get_subscriber_count(None)?;
        let pending_count = self
            .database
            .get_subscriber_count(Some(super::database::SubscriberStatus::Pending))?;
        let approved_count = self
            .database
            .get_subscriber_count(Some(super::database::SubscriberStatus::Approved))?;

        println!("\nSubscriber Statistics:");
        println!("Total Subscribers: {}", total_subscribers);
        println!("Pending Approval: {}", pending_count);
        println!("Approved: {}", approved_count);

        Ok(())
    }
}

/// Interactive IMAP configuration setup
pub fn setup_imap_config() -> Result<ImapConfig> {
    use std::io::{self, Write};

    println!("IMAP Configuration Setup");
    println!("========================");

    print!("IMAP Server (e.g., imap.gmail.com): ");
    io::stdout().flush()?;
    let mut server = String::new();
    io::stdin().read_line(&mut server)?;
    let server = server.trim().to_string();

    print!("IMAP Port (default: 993): ");
    io::stdout().flush()?;
    let mut port_str = String::new();
    io::stdin().read_line(&mut port_str)?;
    let port = if port_str.trim().is_empty() {
        993
    } else {
        port_str.trim().parse().context("Invalid port number")?
    };

    print!("Username (email address): ");
    io::stdout().flush()?;
    let mut username = String::new();
    io::stdin().read_line(&mut username)?;
    let username = username.trim().to_string();

    println!("\nIMAP configuration created!");
    println!("Don't forget to set NEWSLETTER_IMAP_PASSWORD environment variable.");

    Ok(ImapConfig {
        server,
        port,
        username,
        use_tls: Some(true),
    })
}

/// Interactive SMTP configuration setup
#[allow(dead_code)]
pub fn setup_smtp_config() -> Result<SmtpConfig> {
    use std::io::{self, Write};

    println!("SMTP Configuration Setup");
    println!("========================");

    print!("SMTP Server (e.g., smtp.gmail.com): ");
    io::stdout().flush()?;
    let mut server = String::new();
    io::stdin().read_line(&mut server)?;
    let server = server.trim().to_string();

    print!("SMTP Port (default: 587): ");
    io::stdout().flush()?;
    let mut port_str = String::new();
    io::stdin().read_line(&mut port_str)?;
    let port = if port_str.trim().is_empty() {
        587
    } else {
        port_str.trim().parse().context("Invalid port number")?
    };

    print!("Username (email address): ");
    io::stdout().flush()?;
    let mut username = String::new();
    io::stdin().read_line(&mut username)?;
    let username = username.trim().to_string();

    println!("\nSMTP configuration created!");
    println!("Don't forget to set NEWSLETTER_SMTP_PASSWORD environment variable.");

    Ok(SmtpConfig {
        server,
        port,
        username,
        use_tls: Some(true),
    })
}

impl NewsletterManager {
    /// Create a newsletter composer
    pub fn create_composer(
        &self,
        theme: Box<dyn blogr_themes::Theme>,
    ) -> Result<NewsletterComposer> {
        NewsletterComposer::new(theme, self.config.clone())
    }

    /// Create a newsletter sender
    pub fn create_sender(&self, emails_per_minute: Option<u32>) -> Result<NewsletterSender> {
        let smtp_config = self.get_smtp_config()?
            .ok_or_else(|| anyhow::anyhow!(
                "SMTP configuration not found. Set up SMTP config in blogr.toml or environment variables"
            ))?;

        let sender_name = self.config.newsletter.sender_name.clone();
        let rate_limit = emails_per_minute.unwrap_or(10); // Default to 10 emails per minute

        NewsletterSender::new(smtp_config, sender_name, rate_limit)
    }

    /// Compose newsletter from latest blog post
    pub fn compose_from_latest_post(
        &self,
        theme: Box<dyn blogr_themes::Theme>,
        posts: &[Post],
    ) -> Result<Newsletter> {
        let composer = self.create_composer(theme)?;

        let latest_post = posts
            .first()
            .ok_or_else(|| anyhow::anyhow!("No posts found"))?;

        composer.compose_from_post(latest_post)
    }

    /// Send newsletter to all approved subscribers
    pub fn send_newsletter(
        &self,
        newsletter: &Newsletter,
        interactive: bool,
    ) -> Result<super::sender::SendReport> {
        let mut sender = self.create_sender(None)?;

        let password = if interactive {
            self.prompt_for_password("SMTP")?
        } else {
            self.get_smtp_password()?
        };

        let subscribers = self.database.get_subscribers(None)?;

        sender.send_to_subscribers(newsletter, &subscribers, &password, None)
    }

    /// Send test newsletter
    pub fn send_test_newsletter(
        &self,
        newsletter: &Newsletter,
        test_email: &str,
        interactive: bool,
    ) -> Result<()> {
        let mut sender = self.create_sender(None)?;

        let password = if interactive {
            self.prompt_for_password("SMTP")?
        } else {
            self.get_smtp_password()?
        };

        sender.send_test_email(newsletter, test_email, &password)
    }
}
