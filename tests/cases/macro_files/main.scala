package app

import twlib.Tailwind.*

object Styles:
  val base: Tw = tw"flex p-2"
  def row(active: Boolean, extra: Tw): Tw = classNames("grid", "text-sm" -> active, extra, (tw"md:p-3", !active), "hover:flex")
  def chosen(active: Boolean): Tw =
    classNames("grid", if active then tw"p-3" else tw"p-2", (if active then tw"flex" else Tw.empty, true), active match
      case true  => tw"md:flex"
      case false => tw"hover:grid")

@main def run(): Unit =
  println(Styles.base.raw)
  println(Styles.row(true, tw"p-3").raw)
  println(Styles.row(false, Tw.empty).raw)
  println((Styles.base ++ tw"text-sm").raw)
  println(classNames("flex", ("p-2", false)).raw)
  println(Styles.chosen(true).raw)
  println(Styles.chosen(false).raw)
