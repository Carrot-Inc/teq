package shelf

class Stock {
  private type Hidden = Int
  private class Bolted
  protected type Kept = Int
  private[shelf] type Near = Int
  private[shelf] class Fenced
  protected class Guarded
  type Open = String
  class Crate
}
class Outlet extends Stock { type Mine = Int }
trait Ledger { private type Ink = Int; type Page }
class Desk extends Stock with Ledger
type Via = Stock
class Annex extends Via with Ledger { type Extra = Int }
