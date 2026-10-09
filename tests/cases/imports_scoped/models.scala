package app.model

object LoanModels:
  enum ActionType:
    case Create, Cancel, Renew

  case class Loan(id: Int, action: ActionType)

  object Limits:
    val maxItems: Int = 20

object Rules:
  import LoanModels.*
  import ActionType.*

  def default: ActionType = Create
  def flip(a: ActionType) = a match
    case Create => Cancel
    case Cancel => Create
    case Renew => Renew
  def sample: Loan = Loan(1, default)

  import Limits.maxItems
  def capped(n: Int) = if n > maxItems then maxItems else n
