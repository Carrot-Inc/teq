package market

// A trait between an object and `App`, in a file of its own: its statement runs before the object's body.
trait Stall extends App:
  println("stall set up")
