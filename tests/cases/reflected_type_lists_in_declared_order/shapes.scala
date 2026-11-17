package grove

class Ground {
  type Soil
  class Root
}

trait Stem {
  type Sap
}

trait Leaf extends Stem {
  type Vein
  type Edge
}

trait Bark {
  type Ring
}

// Classes and types interleaved, none in the order of its name; `Soil` overrides the parent's.
class Tree[K, V] extends Ground with Leaf with Bark {
  class Nest2
  type Moss2
  class Nest1
  type Moss1 = Int
  type Soil = String
  val height: Int = 1
  def age: Int = 2
}

class Shrub extends Ground {
  type Yew
  type Zest
  type Wax = Int
  class Inner2
  class Inner1
}

object Glade {
  type Pond = Int
  class Nest
  type Lake = String
  class Egg
}

// One member declared by two parents: its owner is the one a lookup finds, the later parent.
trait East { type Sun; type Dawn }
trait West { type Sun; type Dusk }
class Noon extends East with West
class Night extends West with East

// A diamond, the member overridden on one side.
trait Base { type Sun; type Floor }
trait Left extends Base { type Sun; type Port }
trait Right extends Base { type Star }
class Both extends Left with Right
class Other extends Right with Left

// A parent's type parameters are not the child's members.
trait Fourth[A] { type B4 }
trait Third[A] extends Fourth[A] { type B3; type A3 }
trait Second[A] { type B2 }
trait First[A, B, C] extends Second[A] with Third[B] { type B1 }

// A trait whose parent trait extends a class: among the declared parents the class comes first
// (`Namer.ensureFirstIsClass`), so its members stand ahead of the traits'.
class Trunk { type Pith = Int; class Knot }
trait Limb extends Trunk { type Twig = Int }
trait Spray extends Limb { type Bud = Int }
object Copse extends Limb { type Burr = Int }
class Hedge extends Spray { type Thorn = Int }
trait Loose { type Mid }
class Thicket extends Loose with Limb

// A member that several parents declare and one defines: the defined one is the member, and a
// class before an abstract type of its name.
trait Fixed { type Grain = String; type Cz }
trait Open1 { type Grain; type Ab }
trait Open2 { type Grain <: AnyRef; type Ac }
class Board extends Open1 with Fixed with Open2
trait Named { class Plank; type Cc }
trait Bare { type Plank; type Aa }
class Deck extends Named with Bare

// A type parameter is the member of its name: the parent's member of that name is not listed.
trait Holds { type Elem; type Rest }
class Bin[Elem] extends Holds

// A type of an inner object's name stays listed beside the object.
class Hub { object Lamp; type Wick = Int; class Shade; object Shade; type Lamp = Lamp.type }
class Spoke extends Hub

