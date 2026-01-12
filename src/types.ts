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
