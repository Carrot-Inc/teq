// A constant read across modules: a final val, an inline val aliasing it from the next module, and a library
// constant, folded by the third module whether its upstream comes as products or as a jar.
package prodcheck
object U { final val t = 7 }
