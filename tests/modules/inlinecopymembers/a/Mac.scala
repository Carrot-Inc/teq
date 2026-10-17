package ica

// The classes an inline body makes, which each expansion makes in the module of its site:
// `Pair`'s abstract members are extensions, and `bonus` is a member no caller names.
trait Pair:
  extension (x: Int)
    def live: Int
    def dead: Int

trait Wrap:
  def pair: Pair

// The control: ordinary abstract members, which a module's products kept before.
trait Plain:
  def live: Int
  def dead: Int

object Mac:
  inline def anon: Pair = new Pair:
    extension (x: Int)
      def live: Int = x + 1
      def dead: Int = x + 2
    def bonus: Int = 3
    private def secret: Int = 4

  inline def local: Pair =
    class Box(base: Int) extends Pair:
      def this() = this(10)
      extension (x: Int)
        def live: Int = x + base
        def dead: Int = x + 2 * base
      def bonus: Int = 30
    new Box(10)

  inline def nested: Pair = new Wrap:
    def pair: Pair = new Pair:
      extension (x: Int)
        def live: Int = x + 4
        def dead: Int = x + 5
      def bonus: Int = 6
  .pair

  inline def plain: Plain = new Plain:
    def live: Int = 7
    def dead: Int = 8

  // A class the body declares is the expansion's whether it makes it or not; one of a branch
  // the expansion drops is none.
  inline def withUnused: Pair =
    class Unused:
      def bonus: Int = 99
    new Pair:
      extension (x: Int)
        def live: Int = x + 11
        def dead: Int = x + 12

  inline def pick(inline first: Boolean): Pair =
    inline if first then
      class Chosen extends Pair:
        extension (x: Int)
          def live: Int = x + 21
          def dead: Int = x + 22
      new Chosen
    else
      class Dropped extends Pair:
        extension (x: Int)
          def live: Int = x + 31
          def dead: Int = x + 32
      new Dropped
