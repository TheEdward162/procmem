import os

import decky_plugin

class Plugin:
	# A normal method. It can be called from JavaScript using call_plugin_function("method_1", argument1, argument2)
	async def add(self, left, right):
		return left + right

	# Asyncio-compatible long-running code, executed in a task when the plugin is loaded
	async def _main(self):
		decky_plugin.logger.info("Hello World!")

	# Function called first during the unload process, utilize this to handle your plugin being removed
	async def _unload(self):
		decky_plugin.logger.info("Goodbye World!")
		pass
