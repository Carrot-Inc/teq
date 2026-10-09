package vca

class Cents(val amount: Long) extends AnyVal:
  def +(o: Cents): Cents = new Cents(amount + o.amount)
  def show: String = amount.toString + "c"

object Prices:
  def total(xs: List[Cents]): Cents = xs.foldLeft(new Cents(0))(_ + _)
