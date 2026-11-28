object Lines:
  def f(value: Boolean): Boolean = value

  def argumentAnd(a: Boolean, b: Boolean): Boolean =
    f(
      a && b)

  def branchAnd(a: Boolean, b: Boolean): Int =
    if f(
      a && b)
    then 1
    else 2

  def conditionAnd(a: Boolean, b: Boolean): Int =
    if
      a && b
    then 1
    else 2

  def argumentNot(c: Boolean): Boolean =
    f(
      !c)

  def conditionNot(c: Boolean): Int =
    if
      !c
    then 1
    else 2

  def argumentEqual(x: Int, y: Int): Boolean =
    f(
      x == y)

  def branchEqual(x: Int, y: Int): Int =
    if f(
      x == y)
    then 1
    else 2

  def conditionEqual(x: Int, y: Int): Int =
    if
      x == y
    then 1
    else 2

object Main:
  def main(args: Array[String]): Unit =
    println(Lines.argumentAnd(true, true))
    println(Lines.argumentAnd(true, false))
    println(Lines.argumentAnd(false, true))
    println(Lines.branchAnd(true, true))
    println(Lines.branchAnd(true, false))
    println(Lines.branchAnd(false, true))
    println(Lines.conditionAnd(true, true))
    println(Lines.conditionAnd(true, false))
    println(Lines.conditionAnd(false, true))
    println(Lines.argumentNot(true))
    println(Lines.argumentNot(false))
    println(Lines.conditionNot(true))
    println(Lines.conditionNot(false))
    println(Lines.argumentEqual(2, 2))
    println(Lines.argumentEqual(2, 3))
    println(Lines.branchEqual(2, 2))
    println(Lines.branchEqual(2, 3))
    println(Lines.conditionEqual(2, 2))
    println(Lines.conditionEqual(2, 3))
