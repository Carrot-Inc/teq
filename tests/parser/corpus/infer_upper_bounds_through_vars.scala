// An argument whose expected type is a variable bounded above by another variable that has a
// concrete bound: `Some(CashFee(..))` for `orElse[B >: A]` against an `Option[CashFee]` keeps
// the enum case's type, and `Some(n / 100.0)` inside `flatMap[B]` against an
// `Option[BigDecimal]` converts the `Double`, both through `A <: B <: bound`.
enum ItemFee:
  case CreditsFee(value: Int)
  case CashFee(value: BigDecimal)

final case class Item(price: ItemFee.CashFee, unwaivedFee: Option[ItemFee.CashFee])

def fix(item: Item): Item =
  item.copy(unwaivedFee = item.unwaivedFee.orElse(Some(ItemFee.CashFee(item.price.value))))

final case class Row(totalWaived: Option[BigDecimal])
def waive(o: Option[List[Int]]): Row =
  Row(totalWaived = o.flatMap(ds =>
    if ds.isEmpty then None
    else Some(ds.sum / 100.0)
  ))

@main def main(): Unit =
  println(fix(Item(ItemFee.CashFee(1), None)))
  println(waive(Some(List(250))))
  println(waive(Some(Nil)))
