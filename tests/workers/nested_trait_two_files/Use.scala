abstract class Crate extends Stock.Slot
abstract class Pallet extends Stock.Slot with Stock.Sized

class Other(val rack: Rack):
  abstract class Bin extends rack.Slot
  class Box extends rack.Sized

@main def run(): Unit =
  val other = new Other(Stock)
  println(new other.Box().width)
