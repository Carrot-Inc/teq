// The library side of tests/classpath/js/library_object_vals, compiled by scalac there: objects whose load runs
// an initialiser or a statement, their own or a base's (a val, a private val, a var, a pattern val, a lazy val,
// a trait's val, private val, var and statement, a class's val and statement, a trait's val two levels up, an
// overriding val, a trait's argument, an enum's val, a given), and two whose val is read after the load.
package valslib

def note(s: String): Int = { println("  " + s); 7 }
object OwnVal { val v: Int = note("OwnVal: its val") }
object OwnPrivateVal { private val v: Int = note("OwnPrivateVal: its private val") }
object OwnVar { var v: Int = note("OwnVar: its var") }
object OwnPattern { val (a, b) = (note("OwnPattern: its pattern val"), 2) }
object OwnStatement { println("  OwnStatement: its statement") }
object OwnLazy { lazy val v: Int = note("OwnLazy: its lazy val (never at the load)") }
trait ValTrait { val held: Int = note("TraitVal: the trait's val") }
object TraitVal extends ValTrait
trait PrivateValTrait { private val held: Int = note("TraitPrivateVal: the trait's private val") }
object TraitPrivateVal extends PrivateValTrait
trait VarTrait { var held: Int = note("TraitVar: the trait's var") }
object TraitVar extends VarTrait
trait StatementTrait { println("  TraitStatement: the trait's statement") }
object TraitStatement extends StatementTrait
class ValClass { val kept: Int = note("ClassVal: the class's val") }
object ClassVal extends ValClass
class StatementClass { println("  ClassStatement: the class's statement") }
object ClassStatement extends StatementClass
trait UpperTrait { val upper: Int = note("TraitUp: a trait's val two levels up") }
trait MidTrait extends UpperTrait
object TraitUp extends MidTrait
abstract class AbstractHolder { val slot: Int }
object Overrider extends AbstractHolder { val slot: Int = note("Overrider: its overriding val") }
trait ParamTrait(val p: Int)
object TraitParam extends ParamTrait(note("TraitParam: the trait's argument"))
enum Shade { case Dark, Light; val weight: Int = note("Shade: an enum's val, each case") }
object GivenHolder { given held: Int = note("GivenHolder: its given (never at the load)") }
object LateRead { val v: Int = note("LateRead: its val, read after the load") }
trait LateTrait { val held: Int = note("LateTraitRead: the trait's val, read after the load") }
object LateTraitRead extends LateTrait
