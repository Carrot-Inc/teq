package ema

import scala.quoted.*

// The classes a quote defines, which an expansion makes in the module of its site: each
// implements the abstract extensions of `Pair`, and `bonus` is a member no caller names.
trait Pair:
  extension (x: Int)
    def live: Int
    def dead: Int

trait Wrap:
  def pair: Pair

object Shapes:
  inline def anon: Pair = ${ anonImpl }
  inline def local: Pair = ${ localImpl }
  inline def solo: Pair = ${ soloImpl }
  inline def given_ : Pair = ${ givenImpl }
  inline def nested: Pair = ${ nestedImpl }
  inline def kept: Class[?] = ${ keptImpl }
  inline def unused: Pair = ${ unusedImpl }
  inline def outer: AnyRef = ${ outerImpl }
  inline def discarded: Int = ${ discardedImpl }
  inline def holder: Pair = ${ holderImpl }
  inline def lister: AnyRef = ${ listerImpl }
  inline def lifted: AnyRef = ${ liftedImpl }

  def anonImpl(using Quotes): Expr[Pair] =
    '{ new Pair { extension (x: Int) { def live: Int = x + 1; def dead: Int = x + 2 }; def bonus: Int = 3; private def secret: Int = 4 } }
  def localImpl(using Quotes): Expr[Pair] =
    '{ class Box extends Pair { extension (x: Int) { def live: Int = x + 10; def dead: Int = x + 20 }; def bonus: Int = 30 }; new Box }
  def soloImpl(using Quotes): Expr[Pair] =
    '{ object Solo extends Pair { extension (x: Int) { def live: Int = x + 100; def dead: Int = x + 200 }; def bonus: Int = 300 }; Solo }
  def givenImpl(using Quotes): Expr[Pair] =
    '{ given box: Pair with { extension (x: Int) { def live: Int = x + 1000; def dead: Int = x + 2000 }; def bonus: Int = 3000 }; box }
  def nestedImpl(using Quotes): Expr[Pair] =
    '{ new Wrap { def pair: Pair = new Pair { extension (x: Int) { def live: Int = x + 4; def dead: Int = x + 5 }; def bonus: Int = 6 } }.pair }
  // A class the quote declares is the site's whether a run makes it or names it alone.
  def keptImpl(using Quotes): Expr[Class[?]] =
    '{ class Kept { def bonus: Int = 99 }; classOf[Kept] }
  def unusedImpl(using Quotes): Expr[Pair] =
    '{ class Unused { def bonus: Int = 99 }; new Pair { extension (x: Int) { def live: Int = x + 7; def dead: Int = x + 8 } } }
  def outerImpl(using Quotes): Expr[AnyRef] =
    '{ class Outer { class Inner { def bonus: Int = 99 }; def live: Int = 9 }; new Outer }
  // The classes a named class's methods declare and construct are the site's with it.
  def holderImpl(using Quotes): Expr[Pair] =
    '{ class Holder { def pair: Pair = new Pair { extension (x: Int) { def live: Int = x + 13; def dead: Int = x + 14 }; def bonus: Int = 15 } }; new Holder().pair }
  def listerImpl(using Quotes): Expr[AnyRef] =
    '{ class Lister { def kind: Class[?] = { class Hidden { def bonus: Int = 99 }; classOf[Hidden] } }; new Lister().kind }
  // A member whose signature names a type of `scala.quoted` runs later as any other: `Expr` is a class.
  def liftedImpl(using Quotes): Expr[AnyRef] =
    '{ new Object { def bonus(x: Expr[Int]): Int = 16; def none: Option[Expr[Int]] = None } }
  // A quote the macro evaluates and drops declares nothing at the site.
  def discardedImpl(using Quotes): Expr[Int] =
    val dropped = '{ class Discarded { def bonus: Int = 99 }; 0 }
    Expr(1)
