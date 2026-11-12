// The same pattern in the argument of an application whose function does not resolve.
// Pending: A2's erroneous-application dispatch (teq is to report no "not found:
// Missing" at the retained argument) and the not-found message's text (NotFoundMsg).
def f = Nope.call(1 match { case Missing.value => 1; case _ => 0 })
