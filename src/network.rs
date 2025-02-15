use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::net::SocketAddr;
use crate::{ClusterMessage, ClusterNode};
use std::sync::Arc;
use rkyv::Deserialize;

pub struct NetworkServer {
    node: Arc<ClusterNode>,
    address: SocketAddr,
}

impl NetworkServer {
    pub fn new(node: Arc<ClusterNode>, address: SocketAddr) -> Self {
        Self { node, address }
    }

    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let listener = TcpListener::bind(self.address).await?;
        println!("Server listening on {}", self.address);

        loop {
            let (socket, peer_addr) = listener.accept().await?;
            println!("Accepted connection from {}", peer_addr);
            
            let node = Arc::clone(&self.node);
            tokio::spawn(async move {
                if let Err(e) = handle_connection(socket, node).await {
                    eprintln!("Error handling connection from {}: {}", peer_addr, e);
                }
            });
        }
    }
}

async fn handle_connection(
    mut socket: TcpStream,
    node: Arc<ClusterNode>
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut buf = vec![0; 1024];
    
    loop {
        let n = socket.read(&mut buf).await?;
        if n == 0 {
            return Ok(());
        }

        // Deserialize using rkyv
        let archived = unsafe { rkyv::archived_root::<ClusterMessage>(&buf[..n]) };
        let message = archived.deserialize(&mut rkyv::Infallible)?;
        
        // Handle the message and get the response
        let response = node.handle_message(node.info.id, message).await;
        
        // If there's a response, serialize and send it
        if let Some(resp) = response {
            let serialized = rkyv::to_bytes::<_, 1024>(&resp)?;
            socket.write_all(&serialized).await?;
        }
    }
}

pub struct NetworkClient {
    stream: TcpStream,
}

impl NetworkClient {
    pub async fn connect(address: SocketAddr) -> Result<Self, Box<dyn std::error::Error>> {
        let stream = TcpStream::connect(address).await?;
        Ok(Self { stream })
    }

    pub async fn send(&mut self, message: ClusterMessage) -> Result<Option<ClusterMessage>, Box<dyn std::error::Error>> {
        // Serialize and send the message
        let serialized = rkyv::to_bytes::<_, 1024>(&message)?;
        self.stream.write_all(&serialized).await?;
        
        // Read the response
        let mut buf = vec![0; 1024];
        let n = self.stream.read(&mut buf).await?;
        
        if n == 0 {
            Ok(None)
        } else {
            // Deserialize using rkyv
            let archived = unsafe { rkyv::archived_root::<ClusterMessage>(&buf[..n]) };
            let response = archived.deserialize(&mut rkyv::Infallible)?;
            Ok(Some(response))
        }
    }
} 