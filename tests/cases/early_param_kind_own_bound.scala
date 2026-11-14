// A class applied inside the bound of its own higher-kinded parameter (`class Ring[F[X] <: { def self: Ring[F] }]`):
// the check of that application reads the parameter's bound when it runs, and the parameter conforms to it by the
// member its own bound declares. The same through a subclass written before its parent, with another member
// beside, two parameters each applied in the other's bound, an alias, a lower bound, a trait and a method's
// parameter. scalac 3.8.4 prints the same line.
class Sub[F[X] <: { def self: Sub[F] }] extends Ring[F]
class Ring[F[X] <: { def self: Ring[F] }] { def tag: Int = 1 }
class Link[X] { def self: Sub[Link] = new Sub[Link] }

class Shelf[F[X] <: { def self: Shelf[F]; def size: Int }] { def tag: Int = 2 }
class Pile[X] { def self: Shelf[Pile] = new Shelf[Pile]; def size: Int = 3 }

class Pair[F[X] <: { def o: Pair[G, F] }, G[X] <: { def o: Pair[F, G] }] { def tag: Int = 3 }
class Uno[X] { def o: Pair[Uno, Uno] = new Pair[Uno, Uno] }

class Duo[F[X] <: { def g: G[X] }, G[X] <: { def f: F[X] }] { def tag: Int = 4 }
class Fst[X] { def g: Snd[X] = new Snd[X] }
class Snd[X] { def f: Fst[X] = new Fst[X] }

type Again[G[X] <: { def self: Hoop[G] }] = Hoop[G]
class Hoop[F[X] <: { def self: Again[F] }] { def again: Again[F] = this; def tag: Int = 5 }
class Knot[X] { def self: Hoop[Knot] = new Hoop[Knot] }

class Low[F[X] >: Null <: { def self: Low[F] }] { def tag: Int = 6 }
class Tip[X] { def self: Low[Tip] = new Low[Tip] }

trait Line[F[X] <: { def self: Line[F] }] { def tag: Int = 7 }
class Dot[X] { def self: Line[Dot] = new Line[Dot] {} }

def tagOf[F[X] <: { def self: Hold[F] }](h: Hold[F]): Int = h.tag
class Hold[F[X] <: { def self: Hold[F] }] { def tag: Int = 8 }
class Peg[X] { def self: Hold[Peg] = new Hold[Peg] }

@main def main(): Unit =
  println(List(new Sub[Link].tag, new Link[Int].self.tag, new Pile[Int].self.tag, new Uno[Int].o.tag,
    new Duo[Fst, Snd].tag, new Hoop[Knot].again.tag, new Tip[Int].self.tag, new Dot[Int].self.tag,
    tagOf(new Hold[Peg])).mkString(" "))
