package tfa

// A trait's fields of every kind, which a class mixing it in lays out alike whether the trait is
// a source of its build or read from products: a val, a def between them, a var, a private val,
// a lazy val, a constant.
trait Fields:
  val first: Int = 1
  def between: Int = first + 1
  var count: Int = 0
  private val hidden: String = "h"
  lazy val late: Int = { println("late"); 4 }
  final val constant = 5
  def show: String = s"$first $count $hidden $constant"

trait More:
  val more: String = "m"
  var flag: Boolean = true
