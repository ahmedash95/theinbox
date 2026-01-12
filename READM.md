# InboxCleanup

This is a simple macos app built on top of RUST. it suppose to use the macos mail api to read the emails and have an action to mark them as read in bulk.


### The Problem

In my work email, there are several recurring emails that I'm not interested in it. few emails about marketing campaigns we are running. or few notification emails from facebook. such emails are just taking time from my day just to mark them as read. while I could spend this time on emails that matter the most for me

### Solution

What I have in mind is to have this app listing all emails filter them out based on few regex patterns in Subject, From, or even part of the content. after I define this list. the main window of the app shows to me out of non read emails the filtered ones so I can quickly take a look on the subjects list and based on this mark them all as read or adjusting my patterns until I clean it up
