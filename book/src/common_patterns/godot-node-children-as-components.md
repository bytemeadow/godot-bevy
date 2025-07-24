TODO: Explain how to use the ComponentAsGodotNode derive macro.

TODO: Explain this template scene pattern approach to composition with child nodes as bevy components.
- BirdEnemy (Base entity scene)
  - MovementBevyComponent (Private component)
  - Sprite2D (Other private nodes...)
- BirdTemplate (Template scene with public)
  - SpeedBevyComponent (Overridable with editable children)
  - AttackDamageBevyComponent (Overridable with editable children)
  - BirdEnemy (Private implementation details hidden)
- SlowTankBird (BirdEnemy specialization from template)
  - [BirdTemplate] (Editable children enabled)
    - [SpeedBevyComponent] (Configured for slower speed)
    - [AttackDamageBevyComponent] (Configured for extra damage)
    - [BirdEnemy] (Private implementation details hidden)
- FastDartBird (BirdEnemy specialization from template)
  - [BirdTemplate] (Editable children enabled)
    - DashBevyComponent (Additional functionality added)
    - [SpeedBevyComponent] (Configured for faster speed)
    - [AttackDamageBevyComponent] (Configured for reduced damage)
    - [BirdEnemy] (Private implementation details hidden)
- RootScene
  - Player 
  - SlowTankBird
  - FastDartBird