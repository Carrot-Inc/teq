package oz

import magnolia1.*

trait Show[T]:
  def show(t: T): String

object Show extends Derivation[Show]:
  given Show[Int] with
    def show(t: Int): String = t.toString
  given Show[String] with
    def show(t: String): String = t
  def join[T](ctx: CaseClass[Show, T]): Show[T] = new Show[T]:
    def show(t: T): String = ctx.typeInfo.short + ctx.params.map(p => p.label).mkString("(", ",", ")")
  def split[T](ctx: SealedTrait[Show, T]): Show[T] = new Show[T]:
    def show(t: T): String = ctx.typeInfo.owner + "." + ctx.typeInfo.short + ctx.subtypes.map(s => s.typeInfo.short).mkString("[", ",", "]")
