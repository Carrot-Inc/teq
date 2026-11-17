// teq: --wconf msg=A.pure.expression.does.nothing:s
// An object named as a statement is evaluated for its effect: its initialisation runs wherever the object has one
// to the reference (an own statement, an inherited body, a parent's argument, a trait's body), by a bare name and
// through its package; a name whose class has none (its effect in a secondary constructor's argument or an omitted
// default) is removed, and the same object through the package is not.
import objs.*

object Main {
  def main(args: Array[String]): Unit = {
    println("own statement, by name"); OwnN
    println("inherited body, by name"); InheritedN
    println("parent's argument, by name"); ParentArgumentN
    println("secondary's argument, by name"); SecondaryN
    println("omitted default, by name"); DefaultN
    println("quiet, by name"); QuietN
    println("lazy val, by name"); LazyN
    println("nested object, by name"); HolderN.In
    println("trait body, by name"); ViaTraitN
    println("through a class with the trait, by name"); ViaClassN
    println("case object's argument, by name"); CaseArgumentN
    println("own statement, through the package"); objs.OwnP
    println("inherited body, through the package"); objs.InheritedP
    println("parent's argument, through the package"); objs.ParentArgumentP
    println("secondary's argument, through the package"); objs.SecondaryP
    println("omitted default, through the package"); objs.DefaultP
    println("quiet, through the package"); objs.QuietP
    println("lazy val, through the package"); objs.LazyP
    println("nested object, through the package"); objs.HolderP.In
    println("trait body, through the package"); objs.ViaTraitP
    println("through a class with the trait, through the package"); objs.ViaClassP
    println("case object's argument, through the package"); objs.CaseArgumentP
    println("end")
  }
}
