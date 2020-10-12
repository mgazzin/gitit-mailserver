# gitit-mailserver

I am reading [Mastering Rust](https://www.amazon.com/dp/B07GVNJ77X/ref=dp_olp_1) and I think it is a very interesting language. So I was thinking to apply this language to a practical need. I am using [Gitit](https://github.com/jgm/gitit) in my home network and have a personal wiki where I save some important notes. I have access to my personal network with VPN when I am out of home but this requires some time to be activated. So I was thinking to use emails as a gateway to add more content and links to my Gitit wiki pages.
This is simple exercise, however it is a small program that reads one gmail account through IMAP and reads the emails. Email commands should be in the subject and should have the following CSV format:

>command;operation;category;subcategory;link

for example:

>LINK;ADD;ICT;Languages;http://rust-lang.org

At the moment only LINK command and ADD operation are implemented.

## Gmail settings

This programs reads the following environment variables:

```shell
IMAP_HOST=imap.gmail.com
IMAP_PORT=993
IMAP_USERNAME=test-mail@gmail.com
IMAP_PASSWORD=password
```

Please change username and password if you want to use it.
This program requires [Gmail less secure apps](https://support.google.com/accounts/answer/6010255?hl=en) to be applied. 

## Implementation notes

This program uses serde_json and serde_csv.

Any feedback/comment is highly appreciated.
