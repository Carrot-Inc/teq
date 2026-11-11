// expect: method missing overrides nothing
// expect: method toString needs `override` modifier
// expect: method hashCode needs `override` modifier
// expect: method equals of type (other: Any): Int has incompatible type
// expect: value missingVal overrides nothing
// expect: variable missingVar overrides nothing
// expect: type MissingType overrides nothing
package filechecks

override def missing: Int = 1
def toString: String = "holder"
def hashCode(): Int = 1
override def equals(other: Any): Int = 1
override val missingVal: Int = 1
override var missingVar: Int = 2
override type MissingType = Int
