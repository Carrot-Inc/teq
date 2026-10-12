// The nine-line reproduction of macwire's wire[C] of a class with a using clause, its model in a
// file of its own and its wiring in another (macwire_wire_using has it whole).
class Coll
class Api(coll: Coll)(using s: String):
  def show = s"api $s"
