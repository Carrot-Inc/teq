package rto

// The classes whose type-level declarations a macro of the second module lists: classes and types
// interleaved, none in the order of its name, a member that overrides a parent's, parents with
// members of their own and a private type and class, and one member that two parents declare.
class Ground:
  type Soil
  class Root
  private type Deep = Int
  private class Burrow

trait Stem:
  type Sap

trait Leaf extends Stem:
  type Vein
  type Edge

trait Bark:
  type Ring

class Tree[K, V] extends Ground with Leaf with Bark:
  class Nest2
  type Moss2
  class Nest1
  type Moss1 = Int
  type Soil = String
  val height: Int = 1
  def age: Int = 2

class Shrub extends Ground:
  type Yew
  type Zest
  type Wax = Int
  class Inner2
  class Inner1

object Glade:
  type Pond = Int
  class Nest
  type Lake = String
  class Egg

trait East:
  type Sun
  type Dawn

trait West:
  type Sun
  type Dusk

class Noon extends East with West
class Night extends West with East

// A member several parents declare and one defines: the defined one is the member.
trait Fixed:
  type Grain = String
  type Cz

trait Open1:
  type Grain
  type Ab

trait Open2:
  type Grain <: AnyRef
  type Ac

class Board extends Open1 with Fixed with Open2

// A type parameter is the member of its name: the parent's member of that name is not listed.
trait Holds:
  type Elem
  type Rest

class Bin[Elem] extends Holds
