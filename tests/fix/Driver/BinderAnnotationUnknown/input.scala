/*
rules = DisableSyntax
*/
// A binder's annotation is typed (the review of `340567fdd`): an unknown class there is an error of the program,
// which then does not type, and the run is refused.
trait Higher[F[@DoesNotExist A]]
