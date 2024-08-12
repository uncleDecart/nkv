# Notify Key Value Storage (nkv)

### What is it for? 
nkv lets you share state between your services using client server topology. 
it provides you with following API:

- get the state for the given key
- put the state for the given key
- delete the state for the given key
- subscribe to the updates for the given key

### When should I use it?
When you have some shared state between services/processes and you also want to be notified when the value is changed

### How do I use it?

You can create server and then use clients to access it. Server can run in a separate process, thread 

```rust
let temp_dir = TempDir::new().expect("Failed to create temporary directory");

// creates background task where it serves threads
let _srv = Server::new("localhost:4222".to_string(), temp_dir.path().to_path_buf()).await.unwrap();

let nats_url = "localhost:4222".to_string();
let client = NatsClient::new(&nats_url).await.unwrap();

let value: Box<[u8]> = Box::new([1, 2, 3, 4, 5]);
let key = "test_2_key1".to_string();

let resp = client.put(key.clone(), value.clone()).await.unwrap();
assert_eq!(resp, request_msg::ServerResponse::Base(request_msg::BaseResp{
    id: 0,
    status: http::StatusCode::OK,
    message: "No Error".to_string(),
}))
```

### How does it work exactly?

`NotifyKeyValue` is a map of String Key and a value containing `PersistValue` and `Notiffier`.
`PersistValue` is an object, which stores your state (some variable) on file system, so that
you can restart your application without worrying about data loss. `Notifier` on the other hand
handles the channel between server and client to notify latter if anybody changed value. This 
channel is a OS-primitive (currently NATS, which is using sockets under the hood) so you can 
write clients in any programming language you want. Last two components are `Server` and `Client`.
Former creates `NotifyKeyValue` object and manages its access from asynchronous requests. It does 
so by exposing endpoints through some of the OS-primitive (currently NATS), so again, clients could
be in any programming language you like; Latter is used to connect to `Server` and implement the API
(see what is it for section)

You can call those components interfaces, and implement them differently to tailor to your particular 
case, i.e. use mysql for persist value or unix domain socket for `Notfifier` and TCP for `Server` 

### Can I use any other language than Rust?

Yes, underlying design principle is that nkv is using OS primitives, making it possible to write clients in any language.
Just write marshalling and demarshalling in JSON and be sure that you can connect to `Server` and handle `Notifier` (currently NATS)

### Why there's no unsubscribe endpoint?
Current implementation is using NATS as backend, which doesn't require client to explicitly unsubscribe

### Why do you use NATS in `Server` and `Notifier`

### Background
Initially, idea for such design came up in discussions with @deitch talking about improving messaging
system inside [EVE project](https://github.com/lf-edge/eve) called PubSub. We clearly understood one thing:
serverless distributed messaging between services did was not exactly the right way to approach the problem.
What we actually needed was some kind of persistent key-value storage, but with a twist: we want it to notify us when 
there is a change in a value. So all the clients will be able to sync over the value.
Such communication mechanism would be perfect for EVE. In terms of performance the important part for us was
smaller footprint and we also kept in mind that number of connections will be hundreds maximum.

