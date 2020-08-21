![Rangepoll](./logo.png?raw=true)

# Rangepoll
## A web server for voting a poll

Poll can have with multiple choices with scoring.
Decision algorithm is dynamic and configurable.

[Demo](http://rangepoll.herokuapp.com/) login with cyril:cyril


## Why?

When you are in a process of taking important decisions with multiple people, you often want them to express, order, and/or validate their preferences.


This software allows to create and run a poll with all the possible choices and let users express their agreement for each of them with a 5-star score or a binary Yes/No system. 

Poll can have a deadline too.

## Install

This software requires [Rust](https://www.rust-lang.org/tools/install) and so make sure you've installed the Nightly version.

Once it's done, simply run:
```
$ git clone https://github.com/X-Ryl669/rangepoll.git
$ cargo build --release
[... takes a while...]
```

## Usage

### Create polls
Before you can submit polls to your friends, you need to create one like this: 

```
$ target/release/rangepoll -g templates/first_poll.yml
```
The generated poll file is a YAML file that's quite simple to fill (see below for a better description of the format)

### Create voters (optional)
Then you might want to declare some voters, like this:

```
$ target/release/rangepoll -v voters/bob.yml
```
The generated voter file is a YAML file that's very simple to fill. Please notice that the password is in clear in this file (currently there is no security here)

### Create voting tokens (instead of voters)
If you don't want to use voters file, you can ask the software to create voting token for you to dispatch to the voters (for example by email), like this:

```
$ echo "My super secret" > secret.txt
$ target/release/rangepoll -a yourserver.com -t first_poll
Found: "./polls/first_poll.yml"
Voter    Token
John     https://yourserver.com/token/eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJmcnVpdCIsImNvbXBhbnkiOiJKb2huIiwiZXhwIjoxNjAyODU3MTMwfQ.b63cq2I641XPtdxokuTZFOQGkBqSn6zrswGExPn_JVw
Bob      https://yourserver.com/token/eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJmcnVpdCIsImNvbXBhbnkiOiJCb2IiLCJleHAiOjE2MDI4NTcxMzB9.SqHfM6CY3-rq28Am5a8kcLgck0-hxds6D5Jy-BVSvdc
```
Then transmit the tokens to the voters for them to vote directly.

### Run the server

Easy!
```
$ target/release/rangepoll
```
This starts a HTTP server on port 8000 by default, use `-p 80` to use the standard HTTP port.

Please notice if you don't intend to keep the source code of the server, you'll still need the folders (and their subfolders):

- static/
- templates/
- voters/
- polls/
- secret.txt (if using tokens)

In the same directory as the binary.

### Poll file format

A typical poll file is used to describe the vote and to store the results. No database is required and installation is very simple as long as files are R/W on your server.

The poll file format looks like this:
```yaml
---
name: Fruit test
description: This poll is used to test voting algorithms correctly 
# Or you can use: "desc_markdown: file.md" instead for a cleaner presentation
allowed_participant:
  - X
  - Y
  - Z
deadline_date: "2020-08-11 14:05:30"
voting_algorithm: bordat # Any of max, binary, bordat, condorcet, first-choice
choices:
  - name: pear
    description: A pear is good
    # same remark as above, you can use "desc_markdown" to specify a file containing a Markdown description for this choice 
    vote: # Those are generated upon voting
      - 1
      - 2
      - 5
    voter:
      - X
      - Y
      - Z
  - name: apple
    description: An apple a day...
    vote:
      - 2
      - 1
      - 4
    voter:
      - X
      - Y
      - Z
  - name: cherry
    description: Cherry licking...
    vote:
      - 3
      - 3
      - 1
    voter:
      - X
      - Y
      - Z
```
Voting algorithms are described in the `voting_algorithm.html` file

## What isn't this software ?
This software is not a Doodle like system. Users can not add choices to a current poll. 

It isn't made for a country or a huge population (voting algorithms run in N^2 time, but are simple to audit).

It isn't made for anonymous voting and secure counting and hiding of the voter, but instead to provide a tool for running a democratic poll in a company or a gathering of people were the vote results can be public.

## Roadmap
I'm following the [Pareto principle](https://en.wikipedia.org/wiki/Pareto_principle) to save time.

Currently, the software is usable for end-users (voting is working and is a nice UX).

In the future, I'll probably add an admin page to let users change their password (and some security for them instead of storing them in cleartext), and add/delete other users.

Also, I might add a Poll editor (instead of having to deal with a YAML file) since it's not very *non-tech savvy* friendly.




