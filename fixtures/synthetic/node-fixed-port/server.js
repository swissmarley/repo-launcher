// Ignores $PORT on purpose: the Launcher must discover the address from this output.
const express = require('express')
const app = express()
app.get('/', (_req, res) => res.send('<h1>Hello from node-fixed-port</h1>'))
app.listen(38417, '127.0.0.1', () => console.log('Server ready at http://localhost:38417'))
