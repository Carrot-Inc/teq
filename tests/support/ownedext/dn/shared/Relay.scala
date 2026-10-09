package shared

// The downstream's file of the upstream's package calls the member `private[shared]` opens to it.
object Relay:
  def authorized: Int = Id.authorize(41).value
