// expect: method missingLeft overrides nothing
// expect: method missingRight overrides nothing
package left {
  override def missingLeft: Int = 1
}
package right {
  override def missingRight: Int = 2
}
