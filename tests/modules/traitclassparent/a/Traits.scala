package tcpa

// Traits over a class, which the next module reads from this one's products: a class mixing one in extends the
// trait's class parent, as scalac's does.
trait Failure extends Exception
trait Quiet extends scala.util.control.NoStackTrace
