export interface Email {
  id: string;
  message_id: string;
  subject: string;
  sender: string;
  date_received: string;
  mailbox: string;
  account: string;
}

export interface EmailWithMatches extends Email {
  matchingFilters: string[]; // Filter names that matched
}

export type FilterField = "subject" | "sender" | "any";

export interface FilterPattern {
  id: string;
  name: string;
  pattern: string;
  field: FilterField;
  is_regex: boolean;
  enabled: boolean;
}

export interface TestPatternResult {
  match_count: number;
  total_count: number;
  sample_matches: Email[];
}

// Email provider types
export type EmailProvider = "apple_mail" | "gmail";

export interface GmailEmail {
  uid: number;
  message_id: string;
  subject: string;
  sender: string;
  date: string;
}

export interface AppSettings {
  provider: EmailProvider | null;
  gmail_email: string | null;
}

export interface EmailBody {
  html: string | null;
  text: string | null;
}
