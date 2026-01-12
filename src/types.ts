export interface Email {
  id: string;
  message_id: string;
  subject: string;
  sender: string;
  date_received: string;
  date_epoch: number;
  mailbox: string;
  account: string;
  is_read: boolean;
}

export interface EmailWithMatches extends Email {
  matchingFilters: string[]; // Filter names that matched
}

export type FilterField = "subject" | "sender" | "any";

export interface FilterPattern {
  id: number;
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

export interface GmailEmail {
  uid: number;
  message_id: string;
  subject: string;
  sender: string;
  date: string;
  date_epoch: number;
  is_read: boolean;
}

export interface StoredEmail {
  uid: number;
  message_id: string;
  subject: string;
  sender: string;
  date: string;
  date_epoch: number;
  mailbox: string;
  account: string;
  is_read: boolean;
}

export interface AppSettings {
  gmail_email: string | null;
  refresh_interval_minutes?: number;
}

export interface EmailBody {
  html: string | null;
  text: string | null;
}
