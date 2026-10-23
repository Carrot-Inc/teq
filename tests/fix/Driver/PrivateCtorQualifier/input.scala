/*
rules = ["file:../../support/AccessNames.scala"]
*/
// A constructor's `private[Subject]` holds its qualifier, a `Name.Indeterminate` (the pass's PrivateCtor probe).
package driver.privatectorqualifier
class Subject private[Subject] (x: Int) // assert: AccessNames.names
