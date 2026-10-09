package fix.shadow

// A class, a top-level definition and a given that tests/tasty/src-shadow/shadow.scala defines
// again, as two jars can: the class path's first copy is the one scalac reads.
class Shadowed:
  def which: String = "first"

def shadowVersion: String = "first package object"

given defaultShadowed: Shadowed = Shadowed()
