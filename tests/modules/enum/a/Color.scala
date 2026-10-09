package na

enum Color:
  case Red, Green, Blue

enum Planet(val mass: Double):
  case Mercury extends Planet(3.3e23)
  case Earth extends Planet(5.9e24)

enum Opt[+T]:
  case Some(v: T)
  case None
