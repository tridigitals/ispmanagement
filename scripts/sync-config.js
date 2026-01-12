import fs from 'fs';
import path from 'path';

const envPath = path.resolve(process.cwd(), '.env');
const configPath = path.resolve(process.cwd(), 'src-tauri/tauri.conf.json');

function syncConfig() {
    try {
        if (!fs.existsSync(envPath)) {
            console.log('⚠ .env file not found, skipping sync.');
            return;
        }

        // Read .env
        const envContent = fs.readFileSync(envPath, 'utf8');
        const match = envContent.match(/^APP_NAME=["']?([^"'\r\n]+)["']?/m);

        if (!match || !match[1]) {
            console.log('⚠ APP_NAME not found in .env, skipping sync.');
            return;
        }

        const appName = match[1];
        console.log(`ℹ Syncing APP_NAME: "${appName}" to tauri.conf.json...`);

        // Read tauri.conf.json
        const config = JSON.parse(fs.readFileSync(configPath, 'utf8'));

        let changed = false;

        // Update productName
        if (config.productName !== appName) {
            config.productName = appName;
            changed = true;
        }

        // Update window title
        if (config.app && config.app.windows && config.app.windows[0]) {
            if (config.app.windows[0].title !== appName) {
                config.app.windows[0].title = appName;
                changed = true;
            }
        }

        if (changed) {
            fs.writeFileSync(configPath, JSON.stringify(config, null, 2), 'utf8');
            console.log('✓ tauri.conf.json updated successfully.');
        } else {
            console.log('ℹ tauri.conf.json is already up to date.');
        }

    } catch (error) {
        console.error('✖ Error syncing config:', error.message);
    }
}

syncConfig();
