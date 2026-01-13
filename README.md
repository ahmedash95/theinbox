# InboxCleanup

This is a simple macos app built on top of Rust. It uses Gmail IMAP with an App Password to read unread emails and mark them as read in bulk.


### The Problem

In my work email, there are several recurring emails that I'm not interested in it. few emails about marketing campaigns we are running. or few notification emails from facebook. such emails are just taking time from my day just to mark them as read. while I could spend this time on emails that matter the most for me

### Solution

What I have in mind is to have this app list unread emails and filter them out based on a few regex patterns in Subject, From, or even part of the content. After I define this list, the main window of the app shows the filtered unread emails so I can quickly take a look at the subject list and mark them all as read or adjust my patterns until I clean it up.
