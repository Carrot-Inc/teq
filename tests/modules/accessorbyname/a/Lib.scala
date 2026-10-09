package abn

// An inline method that reads a by-name constructor parameter: its accessor evaluates the stored
// thunk at each read, as the class's own read does.
class Lib(x: => Int):
  inline def f: Int = x + x
