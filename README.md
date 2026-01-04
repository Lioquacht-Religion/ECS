# Entity Component System \(ECS\)

## TODO's:
- updating entity dependency graph when adding new components and archetypes
- adding Components to already exiting entity
- removing Components from already exiting entity
- sheduler for running systems in parallel 
- runtime checks for invalid system parameter combinations:
 * multiple queries containing the same mutable components 
 * multiple of the same mutable resource
 * both mutable and shared parameters of a single type of resource or component
- add change detection for entities
- add support for message passing:
 * message writers and reader as system parameters
 * add, remove, change hooks which can be set in the component implementation
 * add setting of hooks at runtime
 and will be executed for their corresponding event
- add option to add system to a specific schedule, e.g. Startup, Update, Cleanup
- add run conditions for systems
- add SparseSet Storage type support
- efficient usage for tag components
- make it possible for resources and query params to be optional, 
by wrapping them inside an option  
