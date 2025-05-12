# WALRus 📚

## Overview 🚀
This project implements a Write Ahead Log (WAL) which ensures durability and consistency by recording changes to the log before they are applied to the actual data store.
## Low-Level Concepts of a Write Ahead Log ⚙️

### 1. **Durability and Atomicity** 🔐
The core purpose of a WAL is to guarantee that all changes to the system are durable. This means that before any changes are made to the database or data store, the operation must be first written to the WAL. If the system crashes before the changes are applied, the log can be replayed to restore the system to its previous consistent state.

### 2. **Sequential Write Operations** 📝
WALs usually consist of a log file that records sequential operations. This means that each new write operation appends to the end of the log, ensuring efficient sequential access. This avoids the need for complex indexing, which is often slow, especially in the case of writes.

### 3. **Atomic Writes** 💥
To ensure consistency, WALs guarantee that each log entry is written atomically. That is, each write is fully written to the log before the actual data store is modified. This atomicity prevents partial writes that could lead to corruption.

### 4. **Log Structure** 🗂️
A typical WAL structure consists of:
   - **Log entries**: Each entry represents a change to the data store, such as an insert, update, or delete operation.
   - **Log files**: These are the actual files where the entries are stored, often on disk for durability.
   - **Checkpointing**: Periodically, a checkpoint is created where the state of the system is consistent with the last log entry. This allows the system to truncate older log entries that have been applied and are no longer necessary.

### 5. **Crash Recovery** ⚠️
If a crash occurs, the WAL is crucial for recovering the system. The logs can be replayed from the last checkpoint to ensure that no data is lost, and the system is brought back to its correct state.

### 6. **Performance Considerations** 🚀
WALs are typically optimized for performance by writing logs sequentially to disk. However, they need to balance durability with speed. Techniques like buffering and batch writing are often used to reduce the overhead of logging operations.

## Phases of the Project 🏗️

### Phase 1: **Single Node Focus** 🔒
In Phase 1, I will focus on implementing a basic single-node WAL system. The goal of this phase is to ensure that the WAL works efficiently on a single machine, providing:
   - Basic functionality for writing logs.
   - Durability guarantees for all write operations.
   - Recovery mechanisms to replay the log and restore the system state after a crash.
   
In this phase, the implementation will primarily target simplicity and correctness, with optimizations and distributed features reserved for future phases.

### Phase 2: **Distributed WAL with Gossip Protocol** 🌐
In Phase 2, the focus will shift towards building a distributed WAL system using a **gossip protocol**. This will allow multiple nodes in a distributed system to synchronize their logs and provide fault tolerance across machines. Key features for this phase include:
   - **Node synchronization**: Nodes will gossip to share WAL entries with each other, ensuring that all nodes have an up-to-date log.
   - **Fault tolerance**: If a node fails, the other nodes can still recover by using the logs they have shared through the gossip protocol.
   - **Log merging and conflict resolution**: Multiple nodes might record different changes to the same data. We will need strategies to resolve conflicts and ensure that the distributed system reaches consensus.

By the end of this phase, the system will have the following characteristics:
   - A fault-tolerant distributed WAL system.
   - Gossip-based synchronization for ensuring consistency.
   - Mechanisms for recovering from node failures by replaying logs.

## Goals and Milestones 🎯

1. **Phase 1 (Single Node)**:
   - Implement basic WAL functionality (write, read, recovery).
   - Ensure durability and atomicity of writes.
   - Perform crash recovery and ensure log consistency.

2. **Phase 2 (Distributed WAL)**:
   - Design and implement a gossip protocol for synchronizing logs across nodes.
   - Build mechanisms for resolving conflicts and ensuring consistency across nodes.
   - Implement fault tolerance and recovery mechanisms in the distributed context.
