import React, { useEffect, useState } from "react";
import { getVersion } from '@tauri-apps/api/app';
import Image from 'next/image';

const FORK_NOTICE = 'Mingtily is an independent community fork of Meetily. It is not affiliated with or endorsed by the Meetily project.';

export function About() {
  const [currentVersion, setCurrentVersion] = useState('0.4.0');

  useEffect(() => {
    getVersion().then(setCurrentVersion).catch(console.error);
  }, []);

  return (
    <div className="p-6 space-y-6 max-h-[80vh] overflow-y-auto">
      <div className="text-center space-y-3">
        <Image
          src="/icon_128x128.png"
          alt="Mingtily logo"
          width={72}
          height={72}
          className="mx-auto"
        />
        <div>
          <h1 className="text-xl font-semibold text-gray-900">Mingtily</h1>
          <p className="text-sm text-gray-500">Version {currentVersion}</p>
        </div>
        <p className="text-sm text-gray-700 leading-relaxed">
          A local-first AI meeting recorder and transcription app. Recordings, transcripts,
          and local models stay on your device by default.
        </p>
      </div>

      <div className="rounded-lg border border-purple-100 bg-purple-50 p-4 space-y-2">
        <h2 className="text-sm font-semibold text-purple-950">Local-first, with optional external AI</h2>
        <p className="text-sm text-purple-900 leading-relaxed">
          External LLM providers remain available as an explicit user choice. When selected,
          transcript content is sent only to the provider you configure for summary generation.
        </p>
      </div>

      <div className="space-y-2 text-sm text-gray-600">
        <p>{FORK_NOTICE}</p>
        <p>
          Mingtily is distributed under the MIT License and includes software and model integrations
          from multiple upstream projects. See LICENSE.md and THIRD_PARTY_NOTICES.md for details.
        </p>
      </div>
    </div>
  );
}
