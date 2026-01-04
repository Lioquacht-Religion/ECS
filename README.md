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
- add support for message passing:
 * message writers and reader as system parameters
 * add, remove, change functions which can be set in the component implementation as constants
 and will be executed for their corresponding event
