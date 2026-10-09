object UseMacroWorkers2:
  def use(x: Any): Boolean = MacroWorkers.isString(x)
