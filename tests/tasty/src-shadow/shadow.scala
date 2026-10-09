package fix.shadow

// The second copy of tests/tasty/src/shadow.scala, in the jar after the fixtures on the class path.
class Shadowed:
  def which: String = "second"
  def onlySecond: Int = 2

def shadowVersion: String = "second package object"

given defaultShadowed: Shadowed = Shadowed()
