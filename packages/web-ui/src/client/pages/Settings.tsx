import React, { useState } from 'react';

interface SettingsState {
  general: {
    theme: 'light' | 'dark' | 'system';
    autoRefresh: boolean;
    refreshInterval: number;
    notifications: boolean;
  };
  agents: {
    maxConcurrent: number;
    timeout: number;
    autoRetry: boolean;
    maxRetries: number;
  };
  worktrees: {
    autoCleanup: boolean;
    cleanupDays: number;
    copyIgnoredFiles: boolean;
  };
  api: {
    baseUrl: string;
    timeout: number;
    retries: number;
  };
}

const Settings: React.FC = () => {
  const [settings, setSettings] = useState<SettingsState>({
    general: {
      theme: 'system',
      autoRefresh: true,
      refreshInterval: 30,
      notifications: true,
    },
    agents: {
      maxConcurrent: 3,
      timeout: 300,
      autoRetry: true,
      maxRetries: 3,
    },
    worktrees: {
      autoCleanup: true,
      cleanupDays: 7,
      copyIgnoredFiles: true,
    },
    api: {
      baseUrl: 'http://localhost:3456',
      timeout: 30,
      retries: 3,
    },
  });

  const [activeTab, setActiveTab] = useState('general');

  const updateSetting = (section: keyof SettingsState, key: string, value: any) => {
    setSettings(prev => ({
      ...prev,
      [section]: {
        ...prev[section],
        [key]: value,
      },
    }));
  };

  const saveSettings = () => {
    // TODO: Implement settings save API call
    console.log('Saving settings:', settings);
  };

  const resetSettings = () => {
    // TODO: Implement settings reset
    console.log('Resetting settings');
  };

  const tabs = [
    { id: 'general', name: 'General', icon: '⚙️' },
    { id: 'agents', name: 'Agents', icon: '🤖' },
    { id: 'worktrees', name: 'Worktrees', icon: '🌳' },
    { id: 'api', name: 'API', icon: '🔌' },
  ];

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="bg-white dark:bg-gray-800 shadow rounded-lg">
        <div className="px-4 py-5 sm:p-6">
          <div className="flex items-center justify-between">
            <div>
              <h2 className="text-lg font-medium text-gray-900 dark:text-white">
                Settings
              </h2>
              <p className="text-sm text-gray-600 dark:text-gray-300 mt-1">
                Configure your CrewChief Web UI preferences
              </p>
            </div>
            <div className="flex space-x-3">
              <button
                onClick={resetSettings}
                className="btn btn-secondary"
              >
                Reset
              </button>
              <button
                onClick={saveSettings}
                className="btn btn-primary"
              >
                Save Changes
              </button>
            </div>
          </div>
        </div>
      </div>

      <div className="bg-white dark:bg-gray-800 shadow rounded-lg">
        <div className="flex">
          {/* Sidebar */}
          <div className="w-64 border-r border-gray-200 dark:border-gray-700">
            <nav className="p-4 space-y-1">
              {tabs.map((tab) => (
                <button
                  key={tab.id}
                  onClick={() => setActiveTab(tab.id)}
                  className={`w-full flex items-center px-3 py-2 text-sm font-medium rounded-md transition-colors ${
                    activeTab === tab.id
                      ? 'bg-primary-100 text-primary-700 dark:bg-primary-900 dark:text-primary-200'
                      : 'text-gray-600 hover:bg-gray-50 hover:text-gray-900 dark:text-gray-300 dark:hover:bg-gray-700 dark:hover:text-white'
                  }`}
                >
                  <span className="mr-3 text-lg">{tab.icon}</span>
                  {tab.name}
                </button>
              ))}
            </nav>
          </div>

          {/* Content */}
          <div className="flex-1 p-6">
            {/* General Settings */}
            {activeTab === 'general' && (
              <div className="space-y-6">
                <div>
                  <h3 className="text-lg font-medium text-gray-900 dark:text-white mb-4">
                    General Settings
                  </h3>
                  
                  <div className="space-y-4">
                    <div>
                      <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">
                        Theme
                      </label>
                      <select
                        value={settings.general.theme}
                        onChange={(e) => updateSetting('general', 'theme', e.target.value)}
                        className="mt-1 input w-full max-w-xs"
                      >
                        <option value="light">Light</option>
                        <option value="dark">Dark</option>
                        <option value="system">System</option>
                      </select>
                    </div>

                    <div className="flex items-center">
                      <input
                        id="auto-refresh"
                        type="checkbox"
                        checked={settings.general.autoRefresh}
                        onChange={(e) => updateSetting('general', 'autoRefresh', e.target.checked)}
                        className="h-4 w-4 text-primary-600 focus:ring-primary-500 border-gray-300 rounded"
                      />
                      <label htmlFor="auto-refresh" className="ml-2 block text-sm text-gray-900 dark:text-white">
                        Enable auto-refresh
                      </label>
                    </div>

                    {settings.general.autoRefresh && (
                      <div>
                        <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">
                          Refresh interval (seconds)
                        </label>
                        <input
                          type="number"
                          min="5"
                          max="300"
                          value={settings.general.refreshInterval}
                          onChange={(e) => updateSetting('general', 'refreshInterval', parseInt(e.target.value))}
                          className="mt-1 input w-full max-w-xs"
                        />
                      </div>
                    )}

                    <div className="flex items-center">
                      <input
                        id="notifications"
                        type="checkbox"
                        checked={settings.general.notifications}
                        onChange={(e) => updateSetting('general', 'notifications', e.target.checked)}
                        className="h-4 w-4 text-primary-600 focus:ring-primary-500 border-gray-300 rounded"
                      />
                      <label htmlFor="notifications" className="ml-2 block text-sm text-gray-900 dark:text-white">
                        Enable notifications
                      </label>
                    </div>
                  </div>
                </div>
              </div>
            )}

            {/* Agent Settings */}
            {activeTab === 'agents' && (
              <div className="space-y-6">
                <div>
                  <h3 className="text-lg font-medium text-gray-900 dark:text-white mb-4">
                    Agent Configuration
                  </h3>
                  
                  <div className="space-y-4">
                    <div>
                      <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">
                        Maximum concurrent agents
                      </label>
                      <input
                        type="number"
                        min="1"
                        max="10"
                        value={settings.agents.maxConcurrent}
                        onChange={(e) => updateSetting('agents', 'maxConcurrent', parseInt(e.target.value))}
                        className="mt-1 input w-full max-w-xs"
                      />
                    </div>

                    <div>
                      <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">
                        Task timeout (seconds)
                      </label>
                      <input
                        type="number"
                        min="60"
                        max="3600"
                        value={settings.agents.timeout}
                        onChange={(e) => updateSetting('agents', 'timeout', parseInt(e.target.value))}
                        className="mt-1 input w-full max-w-xs"
                      />
                    </div>

                    <div className="flex items-center">
                      <input
                        id="auto-retry"
                        type="checkbox"
                        checked={settings.agents.autoRetry}
                        onChange={(e) => updateSetting('agents', 'autoRetry', e.target.checked)}
                        className="h-4 w-4 text-primary-600 focus:ring-primary-500 border-gray-300 rounded"
                      />
                      <label htmlFor="auto-retry" className="ml-2 block text-sm text-gray-900 dark:text-white">
                        Auto-retry failed tasks
                      </label>
                    </div>

                    {settings.agents.autoRetry && (
                      <div>
                        <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">
                          Maximum retries
                        </label>
                        <input
                          type="number"
                          min="1"
                          max="10"
                          value={settings.agents.maxRetries}
                          onChange={(e) => updateSetting('agents', 'maxRetries', parseInt(e.target.value))}
                          className="mt-1 input w-full max-w-xs"
                        />
                      </div>
                    )}
                  </div>
                </div>
              </div>
            )}

            {/* Worktree Settings */}
            {activeTab === 'worktrees' && (
              <div className="space-y-6">
                <div>
                  <h3 className="text-lg font-medium text-gray-900 dark:text-white mb-4">
                    Worktree Management
                  </h3>
                  
                  <div className="space-y-4">
                    <div className="flex items-center">
                      <input
                        id="auto-cleanup"
                        type="checkbox"
                        checked={settings.worktrees.autoCleanup}
                        onChange={(e) => updateSetting('worktrees', 'autoCleanup', e.target.checked)}
                        className="h-4 w-4 text-primary-600 focus:ring-primary-500 border-gray-300 rounded"
                      />
                      <label htmlFor="auto-cleanup" className="ml-2 block text-sm text-gray-900 dark:text-white">
                        Auto-cleanup old worktrees
                      </label>
                    </div>

                    {settings.worktrees.autoCleanup && (
                      <div>
                        <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">
                          Cleanup after (days)
                        </label>
                        <input
                          type="number"
                          min="1"
                          max="30"
                          value={settings.worktrees.cleanupDays}
                          onChange={(e) => updateSetting('worktrees', 'cleanupDays', parseInt(e.target.value))}
                          className="mt-1 input w-full max-w-xs"
                        />
                      </div>
                    )}

                    <div className="flex items-center">
                      <input
                        id="copy-ignored"
                        type="checkbox"
                        checked={settings.worktrees.copyIgnoredFiles}
                        onChange={(e) => updateSetting('worktrees', 'copyIgnoredFiles', e.target.checked)}
                        className="h-4 w-4 text-primary-600 focus:ring-primary-500 border-gray-300 rounded"
                      />
                      <label htmlFor="copy-ignored" className="ml-2 block text-sm text-gray-900 dark:text-white">
                        Copy ignored files to new worktrees
                      </label>
                    </div>
                  </div>
                </div>
              </div>
            )}

            {/* API Settings */}
            {activeTab === 'api' && (
              <div className="space-y-6">
                <div>
                  <h3 className="text-lg font-medium text-gray-900 dark:text-white mb-4">
                    API Configuration
                  </h3>
                  
                  <div className="space-y-4">
                    <div>
                      <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">
                        Base URL
                      </label>
                      <input
                        type="url"
                        value={settings.api.baseUrl}
                        onChange={(e) => updateSetting('api', 'baseUrl', e.target.value)}
                        className="mt-1 input w-full max-w-md"
                        placeholder="http://localhost:3456"
                      />
                    </div>

                    <div>
                      <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">
                        Request timeout (seconds)
                      </label>
                      <input
                        type="number"
                        min="5"
                        max="120"
                        value={settings.api.timeout}
                        onChange={(e) => updateSetting('api', 'timeout', parseInt(e.target.value))}
                        className="mt-1 input w-full max-w-xs"
                      />
                    </div>

                    <div>
                      <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">
                        Request retries
                      </label>
                      <input
                        type="number"
                        min="0"
                        max="10"
                        value={settings.api.retries}
                        onChange={(e) => updateSetting('api', 'retries', parseInt(e.target.value))}
                        className="mt-1 input w-full max-w-xs"
                      />
                    </div>
                  </div>
                </div>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
};

export default Settings;