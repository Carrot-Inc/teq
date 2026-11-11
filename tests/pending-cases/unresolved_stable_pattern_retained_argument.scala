// The same pattern in the argument of an application whose function does not resolve; once a crash. Pending: teq's
// extra "not found: Missing" at the retained argument (A2's erroneous-application dispatch) and the not-found
// message's text (NotFoundMsg).
def f = Nope.call(1 match { case Missing.value => 1; case _ => 0 })
