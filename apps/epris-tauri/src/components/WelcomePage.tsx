import { Plus } from 'lucide-react';

interface WelcomePageProps {
  onCreateFirst: () => void;
}

export function WelcomePage({ onCreateFirst }: WelcomePageProps) {
  return (
    <div className="flex-1 flex items-center justify-center p-10">
      <div className="max-w-xl w-full bg-slate-800/30 border border-slate-700/50 rounded-3xl p-10 text-center shadow-2xl">
        <div className="mx-auto w-20 h-20 rounded-2xl bg-indigo-600/20 border border-indigo-500/30 flex items-center justify-center text-indigo-300 shadow-[0_0_30px_rgba(99,102,241,0.25)]">
          <Plus size={40} />
        </div>
        <h2 className="mt-6 text-2xl font-bold text-white">Create Your First Video</h2>
        <p className="mt-2 text-slate-400 text-sm leading-relaxed">
          Epris will create a new Remotion project workspace for you. You can switch between projects later from the left panel.
        </p>

        <button
          onClick={onCreateFirst}
          className="mt-8 w-full max-w-sm mx-auto px-6 py-4 bg-indigo-600 hover:bg-indigo-500 text-white rounded-2xl font-black text-lg transition-colors shadow-xl shadow-indigo-500/10 active:scale-[0.99]"
        >
          Create Your First Video
        </button>
      </div>
    </div>
  );
}

