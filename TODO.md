TODO
- errors. They are now slightly improved
- split up modules more
- websocket backend
- use multiple clients for better parallel
- investigate vods. website still seems to join irc channel but may just be for sending messages...
- there are too many types named message and the conversions between them are messy. Use a better deserializer for at least the tags and try and remove a layer.
- if we do have uuid as in rfc in code comment, maybe convert to bytes/integer in db.
- explore indexes in db. Index on channel makes things very fast.
- convert to lib. Options should be (at least): dbs {sqlite, postgres, none (return mpsc receiver to user)}, procedure {websocket, irc}. Also means the code needs to handle less configuration
