# Notify Key Value (nkv) keyspace

Notify Key Value provides in-built keyspace functionality with `.` symbol as separator supporting wildcards.
What does that mean? Well, values are stored hierarchically and each level of hierarchy is separated by `.`
And if you want to get all the values stored under certain hierarchy level you can use wildcard, i.e.

#### Restrictions

- keyspace does not work for `put` command

#### Code example

```rust
let url = "localhost:4222".to_string();
let mut client = NkvClient::new(&url);

let value: Box<[u8]> = Box::new([9, 7, 3, 4, 5]);

let resp = client.put("service.topicA.key1".to_string(), value.clone()).await.unwrap();
let resp = client.put("service.topicA.key2".to_string(), value.clone()).await.unwrap();
let resp = client.put("service.topicB.key3".to_string(), value.clone()).await.unwrap();
let resp = client.put("service.topicB.key4".to_string(), value.clone()).await.unwrap();

let get_resp = client.get("service.*").await.unwrap();
// service.topicA.key1
// service.topicA.key2
// service.topicB.key3
// service.topicB.key4
print("{:?}", get_resp);

let get_resp = client.get("service.topicA.*").await.unwrap();
// service.topicA.key1
// service.topicA.key2
print("{:?}", get_resp);

let get_resp = client.get("service.topicA.key1").await.unwrap();
// service.topicA.key1
print("{:?}", get_resp);
```

### Subscriptions

You can't use wildcards (`*` symbol) in subscriptions, however,
when subscribing to a value you automatically subscribe to all it's
possible children, f.e.

```rust
let mut rx = nkv.subscribe("ks1", "uuid1".to_string()).await?;

let new_data: Box<[u8]> = Box::new([5, 6, 7, 8, 9]);
nkv.put("ks1.ks2.ks4", new_data.clone()).await?;

// Will print ks1.ks2.ks4 update
println!(rx.recv().await.unwrap());
```

#### Example

Let's explain different subscriptions with an example. You have a tablespace with the following keys:

```
     a1   a2    a3
    /          /  \ 
   b1         b2  b3
 /    \             
c1    c2
```

The way to think about it is:
- when traversing the tree to create/modify element of the tree whole path will be notified.
- in addition when deleting value all the children are notified.

The following table describes expected behaviours for each type of subscription:

| action | notification |
|---|---|
| create root.a1.b1.c2 leaf | a1, b1 |
| modify root.a1.b1.c2 value | a1, b1 |
| delete root.a1.b1 | a1, b1, c1, c2 |
| modify root.a3.b3 | a3 |

For example when creating `a1.b1.c2`, if you subscribe to either `a1, a1.b1` you will be notified, when deleting `a1.b1` if you subscribe to `a1, a1.b1, a1.b1.c1, a1.b1.c2` you will be notified, and so on
