/*
rules = ["file:../../support/ExtensionBody.scala"]
*/
// A single extension method on its own line is the group's body, no block around it (the pass's SingleExtension
// probe).
package driver.extensionsinglebody
object Subject:
  extension (x: Int) // assert: ExtensionBody.body
    def twice: Int = x + x
