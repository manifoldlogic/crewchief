import * as React from 'react';
import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { Input } from '../../../../src/client/components/ui/input';

describe('Input Component', () => {
  it('renders with default props', () => {
    render(<Input placeholder="Enter text" />);
    const input = screen.getByRole('textbox');
    expect(input).toBeInTheDocument();
    expect(input).toHaveAttribute('placeholder', 'Enter text');
  });

  it('supports different input types', () => {
    const { rerender } = render(<Input type="email" />);
    expect(screen.getByRole('textbox')).toHaveAttribute('type', 'email');

    rerender(<Input type="password" />);
    expect(screen.getByLabelText(/password/i) || screen.getByDisplayValue('')).toHaveAttribute('type', 'password');

    rerender(<Input type="number" />);
    expect(screen.getByRole('spinbutton')).toHaveAttribute('type', 'number');
  });

  it('handles user input correctly', async () => {
    const user = userEvent.setup();
    render(<Input placeholder="Type here" />);
    
    const input = screen.getByRole('textbox');
    await user.type(input, 'Hello, World!');
    
    expect(input).toHaveValue('Hello, World!');
  });

  it('calls onChange when value changes', async () => {
    const handleChange = vi.fn();
    const user = userEvent.setup();
    
    render(<Input onChange={handleChange} />);
    
    const input = screen.getByRole('textbox');
    await user.type(input, 'test');
    
    expect(handleChange).toHaveBeenCalledTimes(4); // Once for each character
  });

  it('is disabled when disabled prop is true', () => {
    render(<Input disabled placeholder="Disabled input" />);
    
    const input = screen.getByRole('textbox');
    expect(input).toBeDisabled();
    expect(input).toHaveClass('disabled:cursor-not-allowed', 'disabled:opacity-50');
  });

  it('supports controlled component pattern', async () => {
    const Component = () => {
      const [value, setValue] = React.useState('');
      return (
        <Input 
          value={value} 
          onChange={(e) => setValue(e.target.value)}
          placeholder="Controlled input"
        />
      );
    };
    
    const user = userEvent.setup();
    render(<Component />);
    
    const input = screen.getByRole('textbox');
    await user.type(input, 'controlled');
    
    expect(input).toHaveValue('controlled');
  });

  it('supports uncontrolled component pattern', () => {
    render(<Input defaultValue="default value" />);
    
    const input = screen.getByRole('textbox');
    expect(input).toHaveValue('default value');
  });

  it('applies custom className', () => {
    render(<Input className="custom-input" />);
    expect(screen.getByRole('textbox')).toHaveClass('custom-input');
  });

  it('has proper focus styles for accessibility', () => {
    render(<Input />);
    const input = screen.getByRole('textbox');
    expect(input).toHaveClass('focus-visible:outline-none', 'focus-visible:ring-2');
  });

  it('supports aria attributes for accessibility', () => {
    render(
      <Input 
        aria-label="Custom label"
        aria-describedby="helper-text"
        aria-required={true}
      />
    );
    
    const input = screen.getByRole('textbox');
    expect(input).toHaveAttribute('aria-label', 'Custom label');
    expect(input).toHaveAttribute('aria-describedby', 'helper-text');
    expect(input).toHaveAttribute('aria-required', 'true');
  });

  it('forwards ref correctly', () => {
    const ref = React.createRef<HTMLInputElement>();
    render(<Input ref={ref} />);
    
    expect(ref.current).toBeInstanceOf(HTMLInputElement);
  });
});