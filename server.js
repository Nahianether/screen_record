const WebSocket = require('ws');
const wss = new WebSocket.Server({ port: 8080, path: '/ws' });

wss.on('connection', function connection(ws) {
  console.log('Client connected.');

  ws.on('message', function message(data, isBinary) {
    if (isBinary) {
      console.log('Received video binary data:', data.length, 'bytes');
    } else {
      console.log('Received text:', data.toString());
    }
    ws.send('Received successfully!');
  });

  ws.on('close', () => {
    console.log('Client disconnected.');
  });
});
