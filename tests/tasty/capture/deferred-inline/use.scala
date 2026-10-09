object UseDeferredInline:
  def viaTransparent(x: Any): Boolean = DeferredInline.viaTransparent(x)
  def viaPlain(x: Any): Boolean = DeferredInline.viaPlain(x)
