// The WebSocket Route
#[post("/ws/{user_id}")]
async fn ws(
    req: HttpRequest,
    body: Payload,
    manager: AppData<WsManager>,
    path: Path<String>
) -> Result<HttpResponse, Error> {
    let user_id = path.into_inner();
    // Update connection to web socket.
    let (response, session, mut msg_stream) = actix_ws::handle(&req, body)?;

    // This handles the sending logic automatically.
    let guard = manager.register(&user_id, session);

    // Spawn the receive loop. YOU have full control here.
    rt::spawn(async move {
        // Move the guard into the task so it lives as long as the connection
        let _keep_alive = guard;

        while let Some(Ok(msg)) = msg_stream.next().await {
            match msg {
                Message::Text(text) => {
                println!("{user_id}: {text}");
                }
                Message::Close(_) => {
                // Break the loop on close. Dropping `_keep_alive` removes the user.
                    break;
                }
                _ => {}
            }
        }
    });

    Ok(response)
}

// Sending to ONE User (from another route)

async fn alert_user(
    manager: AppData<WsManager>,
    path: Path<String>,
) -> Rsp {
    let user_id = path.into_inner();
    
    manager.send_to_user(
        &user_id, 
        AppMessage::SystemAlert("You have a new alert!".to_string())
    );

    send_str(format!("Alert sent to {}", user_id))
}

// Sending to MULTIPLE Users (from another route)
async fn message_group(manager: AppData<WsManager>) -> Rsp {
    let target_users = vec!["user123", "user456", "admin99"];
    // send to multiple
    manager.send_to_users(
        &target_users,
        AppMessage::Notification { 
            title: "Group Update".to_string(), 
            body: "Meeting starts in 5 minutes".to_string() 
        }
    );
    // send to one
    manager.send_to_user(
    "user123",
    AppMessage::DirectMessage{
        from: "user123".to_string(),
        content: "Hi!".to_string(),
    }
    );
    // Broadcast / Send to every connected users
    manager.broadcast(AppMessage::SystemAlert("some".to_string()))
    // Done
    HttpResponse::Ok().body("Group messaged")
}

