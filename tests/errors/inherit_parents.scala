// expect: 31:32: error: class Plant is not a trait
// expect: 32:34: error: class Animal is not a trait
// expect: 33:7: error: class Pebble cannot extend final class Rock
// expect: 34:35: error: case class Point3 has case ancestor class Point, but case-to-case inheritance is prohibited
// expect: 36:21: error: Cyclic inheritance: class Loop2 extends itself
// expect: 38:22: error: missing argument for parameter label
// expect: 39:20: error: missing argument for parameter width
// expect: 39:20: error: missing argument for parameter label
// expect: 40:29: error: type mismatch: found String, required Int
// expect: 40:34: error: type mismatch: found Int, required String
// expect: 41:20: error: too many arguments: expected 2
// expect: 42:27: error: this can be used only in a class, object, or template
// expect: 43:28: error: not found: size
// expect: 45:7: error: class Hammer needs to be abstract, since it has 3 unimplemented members: name, use, weight
// expect: 46:7: error: class Phone needs to be abstract, since def charge(level: Int): String in class Gadget is not defined
// expect: 47:8: error: object creation impossible, since def charge(level: Int): String in class Gadget is not defined
// expect: 49:32: error: illegal trait inheritance: superclass Animal does not derive from trait Grows's superclass Plant
// expect: 52:14: error: Tool is abstract; it cannot be instantiated
// expect: 53:16: error: Walker is a trait; it cannot be instantiated
// expect: 54:16: error: object creation impossible, since def charge(level: Int): String in class Gadget is not defined
// expect: 20 errors found
trait Walker
class Animal
class Plant
final class Rock
case class Point(x: Int)
class Shelf(width: Int, label: String)
abstract class Tool { def use: String; def name(n: Int): String; val weight: Int }
abstract class Gadget { def charge(level: Int): String }

class Both extends Animal with Plant
class Second extends Walker with Animal
class Pebble extends Rock
case class Point3(y: Int) extends Point(y)
class Loop1 extends Loop2
class Loop2 extends Loop1

class Narrow extends Shelf(1)
class Bare extends Shelf
class Swapped extends Shelf("a", 1)
class Wide extends Shelf(1, "a", 2)
class Early extends Shelf(this.size, "a") { val size = 1 }
class Member extends Shelf(size, "a") { val size = 1 }

class Hammer extends Tool
class Phone extends Gadget
object Drill extends Gadget
trait Grows extends Plant
class Weed extends Animal with Grows

object Use:
  val tool = new Tool
  val walker = new Walker
  val gadget = new Gadget {}
