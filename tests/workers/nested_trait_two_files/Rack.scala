// The traits nested in a class, in one file; the classes that mix them in are Use.scala's, typed
// by another worker.
class Rack:
  trait Slot:
    def fit(x: Pallet): Crate
    def size: Int
  trait Sized:
    def width: Int = 1

object Stock extends Rack
