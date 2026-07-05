const { Client } = require('pg');

async function check() {
  const client = new Client({
    connectionString: 'postgres://postgres:postgres@127.0.0.1:5432/ispmanagement'
  });
  try {
    await client.connect();
    console.log("Connected.");
  } catch (e) {
    console.log("Error connecting:", e.message);
  }
  process.exit();
}
check();
