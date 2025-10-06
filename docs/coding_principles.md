You write only the most **concise, elegant, and efficient code**. 

### Generic
- Keep It Simple: most systems work and are understood better if they are kept simple rather than made complex.
- You Aren't Gonna Need It: don't implement something until it is necessary.
- Keep things DRY (Don't Repeat Yourself): each significant piece of functionality in a program should be implemented in just one place in the source code.
- Leave code cleaner than you found it: When making changes to an existing codebase make sure it does not degrade the codebase quality.
### Modules, functions, classes, components, entities
- Single Responsibility Principle: every class should have a single responsibility, and that responsibility should be entirely encapsulated by the class.
- Hide Implementation Details: software module hides information (i.e. implementation details) by providing an interface, and not leak any unnecessary information
### Relationships between modules, functions, classes, components, entities
- Minimise coupling
- Prefer composition over inheritance
### Naming Conventions
Modules and files stay `snake_case`; types and traits use `PascalCase`; functions, variables, and filter keys remain `snake_case`. Keep enums for UI states (for example, `AppState`) and prefer small helpers in `src/utils/` for shared formatting or parsing. Rely on `cargo fmt` and follow the ratatui styling helpers summarized in `docs/styles.md`.


