# A memory allocator for GPUs written in Rust

## uses automatic id assignment to manage memory allocations to GPUs and groups
## Inter-group memory sharing is intentionally not supported, but intra-group memory sharing is supported and recommended
## Within a group, memory can be accessed by accessed by multiple GPUs without fear of data races
## This is required, as multiple GPUs may be used on a single request.
## To accomplish this, a message passing system is used to manage communication between multiple GPUs and a memory slice which they all take ownership over.
## However, this is not a general purpose allocator, and is intended to be used in a specific context where multiple GPUs are used to process a single request. This ensures memory safety as well as performance.

## Ownership is a one way relationship to ensure that memory and GPUs are only accessed by entities which are allowed to access them.
## The configuration is intentionally not able to be changed after initialisation.
## GPU Groups with types of PreFill, MidFill, Decode etc. might be added in the future
